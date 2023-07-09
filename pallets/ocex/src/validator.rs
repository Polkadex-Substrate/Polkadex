use crate::{
	pallet::{Accounts, TriggerRebroadcast, UserActionsBatches, ValidatorSetId},
	settlement::process_trade,
	snapshot::AccountsMap,
	Call, Config, Pallet, ProcessedSnapshotNonce,
};
use frame_system::offchain::SubmitTransaction;
use orderbook_primitives::{
	types::{Trade, UserActionBatch, UserActions, WithdrawalRequest},
	SnapshotSummary,
};
use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::{ingress::IngressMessages, withdrawal::Withdrawal};
use sp_application_crypto::RuntimeAppPublic;
use sp_core::H256;
use sp_runtime::{offchain::storage::StorageValueRef, SaturatedConversion};
use sp_std::vec::Vec;

pub const WORKER_STATUS: [u8; 28] = *b"offchain-ocex::worker_status";
const ACCOUNTS: [u8; 23] = *b"offchain-ocex::accounts";
pub const BATCH: [u8; 20] = *b"offchain-ocex::batch";
const TXN: [u8; 26] = *b"offchain-ocex::transaction";
const LAST_PROCESSED_SNAPSHOT: [u8; 26] = *b"offchain-ocex::snapshot_id";

impl<T: Config> Pallet<T> {
	pub fn run_on_chain_validation(_block_num: T::BlockNumber) -> Result<(), &'static str> {
		let local_keys = T::AuthorityId::all();
		let authorities = Self::validator_set().validators;
		let mut available_keys = authorities
			.into_iter()
			.enumerate()
			.filter_map(move |(_index, authority)| {
				local_keys
					.binary_search(&authority)
					.ok()
					.map(|location| local_keys[location].clone())
			})
			.collect::<Vec<T::AuthorityId>>();
		available_keys.sort();

		if available_keys.is_empty() && sp_io::offchain::is_validator() {
			return Err("No active keys available")
		}

		if <TriggerRebroadcast<T>>::get() {
			let c_info = StorageValueRef::persistent(&TXN);
			match c_info.get::<Call<T>>().map_err(|_| "Unable to decode call")? {
				None => {},
				Some(call) => {
					SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
						.map_err(|_| "Error sending unsigned txn")?;
					return Ok(())
				},
			}
		}

		// Check if another worker is already running or not
		let s_info = StorageValueRef::persistent(&WORKER_STATUS);
		match s_info.get::<bool>().map_err(|err| {
			log::error!(target:"ocex","Error while loading worker status: {:?}",err);
			"Unable to load worker status"
		})? {
			Some(true) => {
				// Another worker is online, so exit
				return Ok(())
			},
			None => {},
			Some(false) => {},
		}
		s_info.set(&true); // Set WORKER_STATUS to true
				   // Check the next ObMessages to process
		let next_nonce = <ProcessedSnapshotNonce<T>>::get().saturating_add(1);

		let snapshot_id_info = StorageValueRef::persistent(&LAST_PROCESSED_SNAPSHOT);
		let last_processed_nonce = match snapshot_id_info
			.get::<u64>()
			.map_err(|_| "Unable to decode last processed snapshot id")?
		{
			None => 0,
			Some(id) => id,
		};

		// Check if we already processed this snapshot and updated our offchain state.
		if last_processed_nonce == next_nonce {
			return Ok(())
		}

		// Load the state to memory
		let s_info = StorageValueRef::persistent(&ACCOUNTS);
		let mut accounts =
			match s_info.get::<AccountsMap>().map_err(|_err| "Unable to get accounts map")? {
				None => AccountsMap::default(),
				Some(acounts) => acounts,
			};

		sp_runtime::print("next_nonce");
		sp_runtime::print(next_nonce);
		sp_runtime::print("last_processed_nonce");
		sp_runtime::print(last_processed_nonce);

		if next_nonce.saturating_sub(last_processed_nonce) > 2 {
			// We need to sync our offchain state
			for nonce in last_processed_nonce.saturating_add(1)..next_nonce {
				// Load the next ObMessages
				let batch = match <UserActionsBatches<T>>::get(nonce) {
					None => {
						log::error!(target:"ocex","No user actions found for nonce: {:?}",nonce);
						return Ok(())
					},
					Some(batch) => batch,
				};
				sp_runtime::print("Processing nonce");
				sp_runtime::print(nonce);
				Self::process_batch(&mut accounts, &batch)?;
			}
		}

		// Load the next ObMessages
		let batch = match <UserActionsBatches<T>>::get(next_nonce) {
			None => {
				log::debug!(target:"ocex","No user actions found for nonce: {:?}",next_nonce);
				s_info.set(&accounts);
				// Store the last processed nonce
				// We need to -1 from next_nonce, as it is not yet processed
				snapshot_id_info.set(&next_nonce.saturating_sub(1));
				return Ok(())
			},
			Some(batch) => batch,
		};

		let withdrawals = Self::process_batch(&mut accounts, &batch)?;

		if sp_io::offchain::is_validator() {
			// Create state hash.
			let state_hash: H256 = H256::from(sp_io::hashing::blake2_256(&accounts.encode()));
			match available_keys.get(0) {
				None => return Err("No active keys found"),
				Some(key) => {
					// Prepare summary
					let summary = SnapshotSummary {
						validator_set_id: <ValidatorSetId<T>>::get(),
						snapshot_id: next_nonce,
						state_hash,
						state_change_id: batch.stid,
						last_processed_blk: accounts.last_block.saturated_into(),
						withdrawals,
						public: key.clone(),
					};
					sp_runtime::print("Summary created!");
					let signature = key.sign(&summary.encode()).ok_or("Private key not found")?;

					let call = Call::submit_snapshot { summary, signature };

					let s_info = StorageValueRef::persistent(&TXN);
					s_info.set(&call); // Store the call for future use

					SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
						.map_err(|_| "Error sending unsigned txn")?;
				},
			}
		}

		s_info.set(&accounts);
		snapshot_id_info.set(&batch.snapshot_id); // Store the processed nonce

		Ok(())
	}

	fn import_blk(blk: T::BlockNumber, state: &mut AccountsMap) -> Result<(), &'static str> {
		log::info!(target:"ocex","Importing block: {:?}",blk);
		if blk <= state.last_block.saturated_into() {
			return Err("BlockOutofSequence")
		}

		let messages = Self::ingress_messages(blk);

		for message in messages {
			// We don't care about any other message
			match message {
				IngressMessages::Deposit(main, asset, amt) => {
					let balances = state
						.balances
						.get_mut(&Decode::decode(&mut &main.encode()[..]).unwrap()) // this conversion will not fail
						.ok_or("Main account not found")?;

					balances
						.entry(asset)
						.and_modify(|total| {
							*total = total.saturating_add(amt);
						})
						.or_insert(amt);
				},
				_ => {},
			}
		}

		state.last_block = blk.saturated_into();

		Ok(())
	}

	fn trades(trades: &Vec<Trade>, state: &mut AccountsMap) -> Result<(), &'static str> {
		log::info!(target:"ocex","Settling trades...");
		for trade in trades {
			let config = Self::trading_pairs(trade.maker.pair.base, trade.maker.pair.quote)
				.ok_or("TradingPairNotFound")?;
			process_trade(state, trade, config)?
		}

		Ok(())
	}

	fn withdraw(
		request: &WithdrawalRequest<T::AccountId>,
		state: &mut AccountsMap,
		stid: u64,
	) -> Result<Withdrawal<T::AccountId>, &'static str> {
		log::info!(target:"ocex","Settling withdraw request...");
		let amount = request.amount().map_err(|_| "decimal conversion error")?;
		let account_info = <Accounts<T>>::get(&request.main).ok_or("Main account not found")?;

		if !account_info.proxies.contains(&request.proxy) {
			// TODO: Check Race condition
			return Err("Proxy not found")
		}
		if !request.verify() {
			return Err("SignatureVerificationFailed")
		}

		let balances = state
			.balances
			.get_mut(&Decode::decode(&mut &request.main.encode()[..]).unwrap()) // This conversion will not fail
			.ok_or("Main account not found")?;

		let total = balances.get_mut(&request.asset()).ok_or("Asset Not found")?;

		if *total < amount {
			return Err("Insufficient Balance")
		}

		*total = total.saturating_sub(amount);

		let withdrawal = request.convert(stid).map_err(|_| "Withdrawal conversion error")?;

		Ok(withdrawal)
	}

	fn process_batch(
		accounts: &mut AccountsMap,
		batch: &UserActionBatch<T::AccountId>,
	) -> Result<Vec<Withdrawal<T::AccountId>>, &'static str> {
		if accounts.stid >= batch.stid {
			return Err("Invalid stid")
		}

		let mut withdrawals = Vec::new();
		// Process Ob messages
		for action in &batch.actions {
			match action {
				UserActions::Trade(trades) => Self::trades(trades, accounts)?,
				UserActions::Withdraw(request) => {
					let withdrawal = Self::withdraw(request, accounts, batch.stid)?;
					withdrawals.push(withdrawal);
				},
				UserActions::BlockImport(blk) =>
					Self::import_blk((*blk).saturated_into(), accounts)?,
				UserActions::Reset => {}, // Not for offchain worker
			}
		}

		Ok(withdrawals)
	}
}
