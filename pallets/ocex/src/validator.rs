// This file is part of Polkadex.
//
// Copyright (c) 2022-2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::{
	aggregator::AggregatorClient,
	lmp::{
		get_fees_paid_by_main_account_in_quote, get_maker_volume_by_main_account,
		get_q_score_and_uptime, store_q_score_and_uptime,
	},
	pallet::{Accounts, FinalizeLMPScore, LMPConfig, ValidatorSetId},
	settlement::{add_balance, get_balance, sub_balance},
	snapshot::StateInfo,
	storage::{store_trie_root, OffchainState},
	Config, Pallet, SnapshotNonce, Snapshots,
};
use frame_system::pallet_prelude::BlockNumberFor;
use num_traits::pow::Pow;
use orderbook_primitives::{
	types::{
		ApprovedSnapshot, Trade, TradingPair, UserActionBatch, UserActions, WithdrawalRequest,
	},
	ObCheckpointRaw, SnapshotSummary,
};
use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::{
	fees::FeeConfig,
	ingress::{EgressMessages, IngressMessages},
	withdrawal::Withdrawal,
	AccountId, AssetId,
};
use rust_decimal::{prelude::Zero, Decimal};
use serde::{Deserialize, Serialize};
use sp_application_crypto::RuntimeAppPublic;
use sp_core::{crypto::ByteArray, H256};
use sp_runtime::{offchain::storage::StorageValueRef, SaturatedConversion};
use sp_std::{borrow::ToOwned, boxed::Box, collections::btree_map::BTreeMap, vec::Vec};
use std::ops::Div;
use trie_db::{TrieError, TrieMut};

/// Key of the storage that stores the status of an offchain worker
pub const WORKER_STATUS: [u8; 28] = *b"offchain-ocex::worker_status";
const STATE_INFO: [u8; 25] = *b"offchain-ocex::state_info";
pub const LAST_PROCESSED_SNAPSHOT: [u8; 26] = *b"offchain-ocex::snapshot_id";
/// Aggregator endpoint: Even though it is centralized for now, it is trustless
/// as it verifies the signature and and relays them to destination.
/// As a future improvment, we can make it decentralized, by having the community run
/// such aggregation endpoints
pub const AGGREGATOR: &str = "https://ob.aggregator.polkadex.trade";
pub const CHECKPOINT_BLOCKS: u64 = 1260;

impl<T: Config> Pallet<T> {
	/// Runs the offchain worker computes the next batch of user actions and
	/// submits snapshot summary to aggregator endpoint
	pub fn run_on_chain_validation(block_num: BlockNumberFor<T>) -> Result<bool, &'static str> {
		let local_keys = T::AuthorityId::all();
		let authorities = Self::validator_set().validators;
		let mut available_keys = authorities
			.iter()
			.enumerate()
			.filter_map(move |(_index, authority)| {
				local_keys
					.binary_search(authority)
					.ok()
					.map(|location| local_keys[location].clone())
			})
			.collect::<Vec<T::AuthorityId>>();

		available_keys.sort();

		if available_keys.is_empty() && sp_io::offchain::is_validator() {
			return Err("No active keys available")
		}

		// Check if another worker is already running or not
		if Self::acquire_offchain_lock().is_err() {
			return Ok(false)
		}
		// Check the next batch to process
		let next_nonce = <SnapshotNonce<T>>::get().saturating_add(1);
		let mut root = crate::storage::load_trie_root();
		log::info!(target:"ocex","block: {:?}, state_root {:?}", block_num, root);
		let mut storage = crate::storage::State;
		let mut state = OffchainState::load(&mut storage, &mut root);
		// Load the state to memory
		let mut state_info = match Self::load_state_info(&mut state) {
			Ok(info) => info,
			Err(err) => {
				log::error!(target:"ocex","Err loading state info from storage: {:?}",err);
				store_trie_root(H256::zero());
				return Err(err)
			},
		};

		let mut last_processed_nonce = state_info.snapshot_id;

		// Check if we already processed this snapshot and updated our offchain state.
		if last_processed_nonce == next_nonce {
			log::debug!(target:"ocex","Submitting last processed snapshot: {:?}",next_nonce);
			// resubmit the summary to aggregator
			AggregatorClient::<T>::load_signed_summary_and_send(next_nonce);
			return Ok(true)
		}

		log::info!(target:"ocex","last_processed_nonce: {:?}, next_nonce: {:?}",last_processed_nonce, next_nonce);

		if next_nonce.saturating_sub(last_processed_nonce) >= CHECKPOINT_BLOCKS {
			log::debug!(target:"ocex","Fetching checkpoint from Aggregator");
			let checkpoint = AggregatorClient::<T>::get_checkpoint();
			// We load a new trie when the state is stale.
			drop(state);
			root = H256::zero();
			storage = crate::storage::State;
			state = OffchainState::load(&mut storage, &mut root);
			let (computed_root, checkpoint) = match checkpoint {
				None => {
					log::error!(target:"ocex","No checkpoint found");
					return Err("No checkpoint found")
				},
				Some(checkpoint) => match Self::process_checkpoint(&mut state, &checkpoint) {
					Ok(_) => {
						// Update params from checkpoint
						Self::update_state_info(&mut state_info, &checkpoint);
						Self::store_state_info(state_info, &mut state);
						let computed_root = state.commit()?;
						(computed_root, checkpoint)
					},
					Err(err) => {
						log::error!(target:"ocex","Error processing checkpoint: {:?}",err);
						return Err("Sync failed")
					},
				},
			};
			log::debug!(target:"ocex","Checkpoint processed: {:?}",checkpoint.snapshot_id);
			let snapshot_summary =
				<Snapshots<T>>::get(checkpoint.snapshot_id).ok_or("Snapshot not found")?;
			if snapshot_summary.state_hash != computed_root {
				log::error!(target:"ocex","State root mismatch: {:?} != {:?}",snapshot_summary.state_hash, computed_root);
				return Err("State root mismatch")
			}
			log::debug!(target:"ocex","State root matched: {:?}",snapshot_summary.state_hash);
			store_trie_root(computed_root);
			last_processed_nonce = snapshot_summary.snapshot_id;
		}
		if next_nonce.saturating_sub(last_processed_nonce) >= 2 {
			if state_info.last_block == 0 {
				state_info.last_block = 4768083; // This is hard coded as the starting point
			}
			// We need to sync our off chain state
			for nonce in last_processed_nonce.saturating_add(1)..next_nonce {
				log::info!(target:"ocex","Syncing batch: {:?}",nonce);
				// Load the next ObMessages
				let batch = match AggregatorClient::<T>::get_user_action_batch(nonce) {
					None => {
						log::error!(target:"ocex","No user actions found for nonce: {:?}",nonce);
						return Ok(true)
					},
					Some(batch) => batch,
				};
				sp_runtime::print("Processing nonce");
				sp_runtime::print(nonce);
				match Self::process_batch(&mut state, &batch, &mut state_info) {
					Ok(_) => {
						state_info.stid = batch.stid;
						state_info.snapshot_id = batch.snapshot_id;
						Self::store_state_info(state_info, &mut state);
						let computed_root = state.commit()?;
						store_trie_root(computed_root);
					},
					Err(err) => {
						log::error!(target:"ocex","Error processing batch: {:?}: {:?}",batch.snapshot_id,err);
						return Err("Sync failed")
					},
				}
			}
		}

		// Load the next ObMessages¡
		log::info!(target:"ocex","Loading user actions for nonce: {:?}",next_nonce);
		let batch = match AggregatorClient::<T>::get_user_action_batch(next_nonce) {
			None => {
				log::debug!(target:"ocex","No user actions found for nonce: {:?}",next_nonce);
				// Store the last processed nonce
				// We need to -1 from next_nonce, as it is not yet processed
				state_info.snapshot_id = next_nonce.saturating_sub(1);
				Self::store_state_info(state_info, &mut state);
				let root = state.commit()?;
				store_trie_root(root);
				log::debug!(target:"ocex","Stored state root: {:?}",root);
				return Ok(true)
			},
			Some(batch) => batch,
		};

		log::info!(target:"ocex","Processing user actions for nonce: {:?}",next_nonce);
		let (withdrawals, egress_messages, trader_metrics) =
			Self::process_batch(&mut state, &batch, &mut state_info)?;

		// Create state hash and store it
		state_info.stid = batch.stid;
		state_info.snapshot_id = batch.snapshot_id; // Store the processed nonce
		Self::store_state_info(state_info, &mut state);
		let state_hash: H256 = state.commit()?;
		store_trie_root(state_hash);
		log::info!(target:"ocex","updated trie root: {:?}", state_hash);

		if sp_io::offchain::is_validator() {
			match available_keys.first() {
				None => return Err("No active keys found"),
				Some(key) => {
					// Unwrap is okay here, we verified the data before.
					let auth_index = Self::calculate_signer_index(&authorities, key)
						.ok_or("Unable to calculate signer index")?;

					// Prepare summary
					let summary = SnapshotSummary {
						validator_set_id: <ValidatorSetId<T>>::get(),
						snapshot_id: next_nonce,
						state_hash,
						state_change_id: batch.stid,
						last_processed_blk: state_info.last_block.saturated_into(),
						withdrawals,
						egress_messages,
						trader_metrics,
					};
					log::debug!(target:"ocex","Summary created by auth index: {:?}",auth_index);
					let signature = key.sign(&summary.encode()).ok_or("Private key not found")?;

					let body = serde_json::to_string(&ApprovedSnapshot {
						summary: summary.encode(),
						index: auth_index.saturated_into(),
						signature: signature.encode(),
					})
					.map_err(|_| "ApprovedSnapshot serialization failed")?;

					if let Err(err) = AggregatorClient::<T>::send_request(
						"submit_snapshot_api",
						&(AGGREGATOR.to_owned() + "/submit_snapshot"),
						body.as_str(),
					) {
						log::error!(target:"ocex","Error submitting signature: {:?}",err);
					}
					store_summary::<T>(summary, signature, auth_index.saturated_into()); // Casting is fine here
				},
			}
		}

		Ok(true)
	}

	/// Checks if another worker is already running or not
	pub fn check_worker_status() -> Result<bool, &'static str> {
		let s_info = StorageValueRef::persistent(&WORKER_STATUS);
		match s_info.get::<bool>().map_err(|err| {
			log::error!(target:"ocex","Error while loading worker status: {:?}",err);
			"Unable to load worker status"
		})? {
			Some(true) => {
				// Another worker is online, so exit
				log::info!(target:"ocex", "Another worker is online, so exit");
				return Ok(false)
			},
			None => {},
			Some(false) => {},
		}
		s_info.set(&true); // Set WORKER_STATUS to true
		Ok(true)
	}

	/// Imports a block into the offchain state and handles the deposits
	fn import_blk(
		blk: BlockNumberFor<T>,
		state: &mut OffchainState,
		state_info: &mut StateInfo,
		engine_messages: &BTreeMap<IngressMessages<T::AccountId>, EgressMessages<T::AccountId>>,
	) -> Result<Vec<EgressMessages<T::AccountId>>, &'static str> {
		log::debug!(target:"ocex","Importing block: {:?}",blk);

		if blk != state_info.last_block.saturating_add(1).into() {
			log::error!(target:"ocex","Last processed blk: {:?},  given: {:?}",state_info.last_block, blk);
			return Err("BlockOutofSequence")
		}

		let messages = Self::ingress_messages(blk);
		let mut verified_egress_messages = Vec::new();

		for message in messages {
			match message {
				IngressMessages::Deposit(main, asset, amt) => add_balance(
					state,
					&Decode::decode(&mut &main.encode()[..])
						.map_err(|_| "account id decode error")?,
					asset,
					amt,
				)?,
				IngressMessages::AddLiquidity(
					market,
					ref pool,
					ref lp,
					total_shares,
					base_deposited,
					quote_deposited,
				) => {
					// Add Base
					add_balance(
						state,
						&Decode::decode(&mut &pool.encode()[..])
							.map_err(|_| "account id decode error")?,
						market.base_asset,
						base_deposited,
					)?;

					// Add Quote
					add_balance(
						state,
						&Decode::decode(&mut &pool.encode()[..])
							.map_err(|_| "account id decode error")?,
						market.quote_asset,
						quote_deposited,
					)?;
					log::debug!(target:"ocex","Added Liquidity for pool:  {:?}/{:?}, by LP: {:?}",market.base_asset, market.quote_asset, lp);
					log::debug!(target:"ocex","Base added: {:?}, Quote added: {:?} LP shares issued:  {:?}",base_deposited, quote_deposited, lp);

					let base_balance = get_balance(
						state,
						&Decode::decode(&mut &pool.encode()[..])
							.map_err(|_| "account id decode error")?,
						market.base_asset,
					)?;

					let quote_balance = get_balance(
						state,
						&Decode::decode(&mut &pool.encode()[..])
							.map_err(|_| "account id decode error")?,
						market.quote_asset,
					)?;

					match engine_messages.get(&message).cloned() {
						None => return Err("Unable to find Egress message for AddLiquidity"),
						Some(engine_result) => {
							if let EgressMessages::AddLiquidityResult(
								pool_e,
								lp_e,
								issued_shares,
								price,
								total_inventory,
							) = &engine_result
							{
								if pool != pool_e {
									return Err("Invalid Pool id in egress")
								}

								if lp != lp_e {
									return Err("Invalid LP address in egress")
								}

								let total_inventory_in_quote = quote_balance
									.saturating_add(price.saturating_mul(base_balance));
								if *total_inventory != total_inventory_in_quote {
									log::error!(target:"ocex","Inventory mismatch: offchain: {:?}, engine: {:?}", total_inventory_in_quote,total_inventory);
									return Err("Inventory Mismatch")
								}

								let given_inventory = base_deposited
									.saturating_mul(*price)
									.saturating_add(quote_deposited);

								let shares_minted = if total_inventory.is_zero() {
									// First LP case
									given_inventory // Since total_inventory is zero, shares = given inventory
								} else {
									given_inventory
										.saturating_mul(total_shares)
										.div(total_inventory)
								};

								if *issued_shares != shares_minted {
									log::error!(target:"ocex","Shares minted: Offchain: {:?}, On-chain: {:?}",shares_minted,issued_shares);
									return Err("Invalid number of LP shares minted")
								}

								// Egress message is verified
								verified_egress_messages.push(engine_result);
							} else {
								return Err("Invalid Engine Egress message")
							}
						},
					}
				},
				IngressMessages::RemoveLiquidity(market, ref pool, ref lp, burn_frac) => {
					let base_balance = get_balance(
						state,
						&Decode::decode(&mut &pool.encode()[..])
							.map_err(|_| "account id decode error")?,
						market.base_asset,
					)?;

					let quote_balance = get_balance(
						state,
						&Decode::decode(&mut &pool.encode()[..])
							.map_err(|_| "account id decode error")?,
						market.quote_asset,
					)?;

					let withdrawing_base = burn_frac.saturating_mul(base_balance);
					let withdrawing_quote = burn_frac.saturating_mul(quote_balance);

					let engine_message = match engine_messages.get(&message) {
						None => return Err("RemoveLiquidity engine message not found"),
						Some(engine_msg) => engine_msg,
					};
					log::error!(target:"ocex", "Engine message for remove liquidity ingress: {:?}",engine_message);
					match engine_message {
						EgressMessages::RemoveLiquidityResult(
							pool_e,
							lp_e,
							base_freed,
							quote_freed,
						) => {
							if pool != pool_e {
								return Err("Invalid Pool id in egress")
							}

							if lp != lp_e {
								return Err("Invalid LP address in egress")
							}

							if withdrawing_quote != *quote_freed {
								log::error!(target:"ocex","Quote Amount: expected: {:?}, freed: {:?}", withdrawing_quote,quote_freed);
								return Err("Invalid quote amount freed!")
							}

							if withdrawing_base != *base_freed {
								log::error!(target:"ocex","Base Amount: expected: {:?}, freed: {:?}", withdrawing_base,base_freed);
								return Err("Invalid base amount freed!")
							}

							// Sub Quote
							sub_balance(
								state,
								&Decode::decode(&mut &pool.encode()[..])
									.map_err(|_| "account id decode error")?,
								market.quote_asset,
								withdrawing_quote,
							)?;

							// Sub Base
							sub_balance(
								state,
								&Decode::decode(&mut &pool.encode()[..])
									.map_err(|_| "account id decode error")?,
								market.base_asset,
								withdrawing_base,
							)?;

							// Egress message is verified
							verified_egress_messages.push(engine_message.clone());
						},
						EgressMessages::RemoveLiquidityFailed(
							pool_e,
							lp_e,
							burn_frac_e,
							base_free,
							quote_free,
							base_required,
							quote_required,
						) => {
							if pool != pool_e {
								return Err("Invalid Pool id in egress")
							}

							if lp != lp_e {
								return Err("Invalid LP address in egress")
							}

							if burn_frac != *burn_frac_e {
								return Err("Invalid Burn fraction in egress")
							}

							if withdrawing_quote != *quote_required {
								log::error!(target:"ocex","Quote Amount: expected: {:?}, required: {:?}", withdrawing_quote,quote_required);
								return Err("Invalid quote amount required by engine!")
							}

							if withdrawing_base != *base_required {
								log::error!(target:"ocex","Base Amount: expected: {:?}, required: {:?}", withdrawing_base,base_required);
								return Err("Invalid base amount required by engine!")
							}

							if withdrawing_quote <= *quote_free {
								log::error!(target:"ocex","Quote Amount: Free Balance: {:?}, required: {:?}", quote_free,withdrawing_quote);
								return Err("Enough quote available but still denied by engine!")
							}

							if withdrawing_base <= *base_free {
								log::error!(target:"ocex","Base Amount: Free Balance: {:?}, required: {:?}", base_free,withdrawing_base);
								return Err(
									"Enough base balance available but still denied by engine!",
								)
							}

							// Egress message is verified
							verified_egress_messages.push(engine_message.clone());
						},
						_ => return Err("Invalid engine message"),
					}
				},
				IngressMessages::ForceClosePool(market, pool) => {
					// Get Balance
					let base_balance = get_balance(
						state,
						&Decode::decode(&mut &pool.encode()[..])
							.map_err(|_| "account id decode error")?,
						market.base_asset,
					)?;

					let quote_balance = get_balance(
						state,
						&Decode::decode(&mut &pool.encode()[..])
							.map_err(|_| "account id decode error")?,
						market.quote_asset,
					)?;

					// Free up all balances
					sub_balance(
						state,
						&Decode::decode(&mut &pool.encode()[..])
							.map_err(|_| "account id decode error")?,
						market.base_asset,
						base_balance,
					)?;

					sub_balance(
						state,
						&Decode::decode(&mut &pool.encode()[..])
							.map_err(|_| "account id decode error")?,
						market.quote_asset,
						quote_balance,
					)?;

					verified_egress_messages.push(EgressMessages::PoolForceClosed(
						market,
						pool,
						base_balance,
						quote_balance,
					));
				},
				_ => {},
			}
		}

		state_info.last_block = blk.saturated_into();
		Ok(verified_egress_messages)
	}

	/// Processes a trade between a maker and a taker, updating their order states and balances
	fn trades(trades: &Vec<Trade>, state: &mut OffchainState) -> Result<(), &'static str> {
		log::info!(target:"ocex","Settling trades...");
		for trade in trades {
			let config = Self::trading_pairs(trade.maker.pair.base, trade.maker.pair.quote)
				.ok_or("TradingPairNotFound")?;
			let (maker_fees, taker_fees) = Self::get_fee_structure(
				&Self::convert_account_id(&trade.maker.main_account)?,
				&Self::convert_account_id(&trade.taker.main_account)?,
			)
			.ok_or("Fee structure not found")?;
			Self::process_trade(state, trade, config, maker_fees, taker_fees)?
		}
		Ok(())
	}

	/// Processes a withdrawal request, updating the account balances accordingly.
	fn withdraw(
		request: &WithdrawalRequest<T::AccountId>,
		state: &mut OffchainState,
		stid: u64,
	) -> Result<Withdrawal<T::AccountId>, &'static str> {
		log::info!(target:"ocex","Settling withdraw request...");
		let amount = request.amount().map_err(|_| "decimal conversion error")?;
		// FIXME: Don't remove these comments, will be reintroduced after fixing the race condition
		// let account_info = <Accounts<T>>::get(&request.main).ok_or("Main account not found")?;

		// if !account_info.proxies.contains(&request.proxy) {
		// 	// TODO: Check Race condition: this is harmless but annoying though
		// 	return Err("Proxy not found")
		// }

		if !request.verify() {
			return Err("SignatureVerificationFailed")
		}
		sub_balance(
			state,
			&Decode::decode(&mut &request.main.encode()[..])
				.map_err(|_| "account id decode error")?,
			request.asset(),
			amount,
		)?;
		let withdrawal = request.convert(stid).map_err(|_| "Withdrawal conversion error")?;

		Ok(withdrawal)
	}

	/// Processes a batch of user actions, updating the offchain state accordingly.
	fn process_batch(
		state: &mut OffchainState,
		batch: &UserActionBatch<T::AccountId>,
		state_info: &mut StateInfo,
	) -> Result<
		(
			Vec<Withdrawal<T::AccountId>>,
			Vec<EgressMessages<T::AccountId>>,
			Option<
				BTreeMap<
					TradingPair,
					(BTreeMap<T::AccountId, (Decimal, Decimal)>, (Decimal, Decimal)),
				>,
			>,
		),
		&'static str,
	> {
		if state_info.stid >= batch.stid {
			return Err("Invalid stid")
		}

		let mut withdrawals = Vec::new();
		let mut egress_messages = Vec::new();
		// Process Ob messages
		for action in &batch.actions {
			match action {
				UserActions::Trade(trades) => Self::trades(trades, state)?,
				UserActions::Withdraw(request) => {
					let withdrawal = Self::withdraw(request, state, 0)?;
					withdrawals.push(withdrawal);
				},
				UserActions::BlockImport(blk, engine_messages) => {
					let mut verified_egress_msgs = Self::import_blk(
						(*blk).saturated_into(),
						state,
						state_info,
						engine_messages,
					)?;
					egress_messages.append(&mut verified_egress_msgs)
				},
				UserActions::Reset => {}, // Not for offchain worker
				UserActions::WithdrawV1(request, stid) => {
					let withdrawal = Self::withdraw(request, state, *stid)?;
					withdrawals.push(withdrawal);
				},
				UserActions::OneMinLMPReport(market, epoch, index, total, scores) => {
					Self::store_q_scores(state, *market, *epoch, *index, *total, scores)?;
				},
			}
		}
		let trader_metrics = Self::compute_trader_metrics(state)?;
		Ok((withdrawals, egress_messages, trader_metrics))
	}

	pub fn store_q_scores(
		state: &mut OffchainState,
		market: TradingPair,
		epoch: u16,
		index: u16,
		total: Decimal,
		scores: &BTreeMap<T::AccountId, Decimal>,
	) -> Result<(), &'static str> {
		for (main, score) in scores {
			store_q_score_and_uptime(
				state,
				epoch,
				index,
				*score,
				&market,
				&Decode::decode(&mut &main.encode()[..]).unwrap(), // unwrap is fine.
			)?;
		}
		Ok(())
	}

	pub fn compute_trader_metrics(
		state: &mut OffchainState,
	) -> Result<
		Option<
			BTreeMap<TradingPair, (BTreeMap<T::AccountId, (Decimal, Decimal)>, (Decimal, Decimal))>,
		>,
		&'static str,
	> {
		// Check if epoch has ended and score is computed if yes, then continue
		if let Some(epoch) = <FinalizeLMPScore<T>>::get() {
			let config =
				<LMPConfig<T>>::get(epoch).ok_or("LMPConfig not defined for this epoch")?;
			let enabled_pairs: Vec<TradingPair> = config.market_weightage.keys().cloned().collect();
			// map( market => (map(account => (score,fees)),total_score, total_fees_paid))
			let mut scores_map: BTreeMap<
				TradingPair,
				(BTreeMap<T::AccountId, (Decimal, Decimal)>, (Decimal, Decimal)),
			> = BTreeMap::new();
			for pair in enabled_pairs {
				let mut map = BTreeMap::new();
				let mut total_score = Decimal::zero();
				let mut total_fees_paid = Decimal::zero();
				// Loop over all main accounts and compute their final scores
				for (main_type, _) in <Accounts<T>>::iter() {
					let main: AccountId = Decode::decode(&mut &main_type.encode()[..]).unwrap();
					let maker_volume =
						get_maker_volume_by_main_account(state, epoch, &pair, &main)?;
					// TODO: Check if the maker volume of this main is greater than 0.25% of the
					// total maker volume in the previous epoch, otherwise ignore this account
					let fees_paid =
						get_fees_paid_by_main_account_in_quote(state, epoch, &pair, &main)?;
					// Get Q_score and uptime information from offchain state
					let (q_score, uptime) = get_q_score_and_uptime(state, epoch, &pair, &main)?;
					let uptime = Decimal::from(uptime);
					// Compute the final score
					let final_score = q_score
						.pow(0.15f64)
						.saturating_mul(uptime.pow(5.0f64))
						.saturating_mul(maker_volume.pow(0.85f64)); // q_final = (q_score)^0.15*(uptime)^5*(maker_volume)^0.85
											// Update the trader map
					map.insert(main_type, (final_score, fees_paid));
					// Compute the total
					total_score = total_score.saturating_add(final_score);
					total_fees_paid = total_fees_paid.saturating_add(fees_paid);
				}
				// Aggregate into a map
				scores_map.insert(pair, (map, (total_score, total_fees_paid)));
			}
			// Store the results so it's not computed again.
			return Ok(Some(scores_map))
		}
		Ok(None)
	}

	/// Processes a checkpoint, updating the offchain state accordingly.
	pub fn process_checkpoint(
		state: &mut OffchainState,
		checkpoint: &ObCheckpointRaw,
	) -> Result<(), &'static str> {
		log::info!(target:"ocex","Processing checkpoint: {:?}",checkpoint.snapshot_id);
		for (account_asset, balance) in &checkpoint.balances {
			let key = account_asset.main.to_raw_vec();
			let mut value = match state.get(&key)? {
				None => BTreeMap::new(),
				Some(encoded) => BTreeMap::decode(&mut &encoded[..])
					.map_err(|_| "Unable to decode balances for account")?,
			};
			value.insert(account_asset.asset, *balance);
			state.insert(key, value.encode());
		}
		Ok(())
	}

	/// Updates the state info
	pub fn update_state_info(state_info: &mut StateInfo, checkpoint: &ObCheckpointRaw) {
		state_info.snapshot_id = checkpoint.snapshot_id;
		state_info.stid = checkpoint.state_change_id;
		state_info.last_block = checkpoint.last_processed_block_number;
		log::debug!(target:"ocex","Updated state_info");
	}

	/// Loads the state info from the offchain state
	pub fn load_state_info(state: &mut OffchainState) -> Result<StateInfo, &'static str> {
		match state.get(&STATE_INFO.to_vec())? {
			Some(data) => Ok(StateInfo::decode(&mut &data[..]).unwrap_or_default()),
			None => Ok(StateInfo::default()),
		}
	}

	/// Stores the state info in the offchain state
	fn store_state_info(state_info: StateInfo, state: &mut OffchainState) {
		state.insert(STATE_INFO.to_vec(), state_info.encode());
	}

	/// Calculates the index of the signer in the authorities array
	fn calculate_signer_index(
		authorities: &[T::AuthorityId],
		expected_signer: &T::AuthorityId,
	) -> Option<usize> {
		let mut auth_index: Option<usize> = None;
		for (index, auth) in authorities.iter().enumerate() {
			if *expected_signer == *auth {
				auth_index = Some(index);
				break
			}
		}
		auth_index
	}

	/// Returns the offchain state
	pub fn get_offchain_balance(
		account: &polkadex_primitives::AccountId,
	) -> Result<BTreeMap<AssetId, Decimal>, &'static str> {
		let mut root = crate::storage::load_trie_root();
		let mut storage = crate::storage::State;
		let state = crate::storage::get_state_trie(&mut storage, &mut root);
		let balance: BTreeMap<AssetId, Decimal> =
			match state.get(account.as_slice()).map_err(crate::validator::map_trie_error)? {
				None => BTreeMap::new(),
				Some(encoded) => BTreeMap::decode(&mut &encoded[..])
					.map_err(|_| "Unable to decode balances for account")?,
			};
		Ok(balance)
	}

	/// Returns the offchain state
	pub fn get_state_info() -> Result<StateInfo, &'static str> {
		let mut root = crate::storage::load_trie_root();
		let mut storage = crate::storage::State;
		let mut state = OffchainState::load(&mut storage, &mut root);
		Self::load_state_info(&mut state)
	}

	/// Returns the FeeConfig from runtime for maker and taker
	pub fn get_fee_structure(
		maker: &T::AccountId,
		taker: &T::AccountId,
	) -> Option<(FeeConfig, FeeConfig)> {
		// TODO: Read this from offchain state to avoid a race condition
		let maker_config = match <Accounts<T>>::get(maker) {
			None => return None,
			Some(x) => x.fee_config,
		};

		let taker_config = match <Accounts<T>>::get(taker) {
			None => return None,
			Some(x) => x.fee_config,
		};

		Some((maker_config, taker_config))
	}

	fn convert_account_id(acc: &AccountId) -> Result<T::AccountId, &'static str> {
		Decode::decode(&mut &acc.encode()[..]).map_err(|_| "Unable to decode decimal")
	}
}

/// Stores the summary in the storage
fn store_summary<T: Config>(
	summary: SnapshotSummary<T::AccountId>,
	signature: <<T as Config>::AuthorityId as RuntimeAppPublic>::Signature,
	auth_index: u16,
) {
	let mut key = LAST_PROCESSED_SNAPSHOT.to_vec();
	key.append(&mut summary.snapshot_id.encode());
	let summay_ref = StorageValueRef::persistent(&key);
	summay_ref.set(&(summary, signature, auth_index));
}

/// Helper function to map trie error to a static str
#[allow(clippy::boxed_local)]
pub fn map_trie_error<T, E>(err: Box<TrieError<T, E>>) -> &'static str {
	match *err {
		TrieError::InvalidStateRoot(_) => "Invalid State Root",
		TrieError::IncompleteDatabase(_) => "Incomplete Database",
		TrieError::ValueAtIncompleteKey(_, _) => "ValueAtIncompleteKey",
		TrieError::DecoderError(_, _) => "DecoderError",
		TrieError::InvalidHash(_, _) => "InvalidHash",
	}
}

/// Http Resposne body
#[derive(Serialize, Deserialize)]
pub struct JSONRPCResponse {
	jsonrpc: serde_json::Value,
	pub result: Vec<u8>,
	id: u64,
}

impl JSONRPCResponse {
	pub fn new(content: Vec<u8>) -> Self {
		Self { jsonrpc: "2.0".into(), result: content, id: 2 }
	}
}
