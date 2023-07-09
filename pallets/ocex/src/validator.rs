use crate::{
	pallet::{Accounts, TriggerRebroadcast, UserActionsBatches, ValidatorSetId},
	settlement::{add_balance, process_trade, sub_balance},
	snapshot::StateInfo,
	storage::store_trie_root,
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
use sp_runtime::{offchain::storage::StorageValueRef, traits::BlakeTwo256, SaturatedConversion};
use sp_std::{boxed::Box, vec::Vec};
use sp_trie::{LayoutV1, TrieDBMut};
use trie_db::{TrieError, TrieMut};

pub const WORKER_STATUS: [u8; 28] = *b"offchain-ocex::worker_status";
const STATE_INFO: [u8; 25] = *b"offchain-ocex::state_info";
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

		// Load the state to memory
		let mut root = crate::storage::load_trie_root();
		let mut storage = crate::storage::State;
		let mut state = crate::storage::get_state_trie(&mut storage, &mut root);

		let mut state_info = Self::load_state_info(&state);

		let _snapshot_id_info = StorageValueRef::persistent(&LAST_PROCESSED_SNAPSHOT);
		let last_processed_nonce = state_info.snapshot_id;

		// Check if we already processed this snapshot and updated our offchain state.
		if last_processed_nonce == next_nonce {
			return Ok(())
		}

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
				Self::process_batch(&mut state, &batch, &mut state_info)?;
			}
		}

		// Load the next ObMessages
		let batch = match <UserActionsBatches<T>>::get(next_nonce) {
			None => {
				log::debug!(target:"ocex","No user actions found for nonce: {:?}",next_nonce);
				// Store the last processed nonce
				// We need to -1 from next_nonce, as it is not yet processed
				state_info.snapshot_id = next_nonce.saturating_sub(1);
				Self::store_state_info(state_info, &mut state)?;
				state.commit();
				store_trie_root(*state.root());
				return Ok(())
			},
			Some(batch) => batch,
		};

		let withdrawals = Self::process_batch(&mut state, &batch, &mut state_info)?;

		if sp_io::offchain::is_validator() {
			// Create state hash.
			let state_hash: H256 = *state.root();
			match available_keys.get(0) {
				None => return Err("No active keys found"),
				Some(key) => {
					// Prepare summary
					let summary = SnapshotSummary {
						validator_set_id: <ValidatorSetId<T>>::get(),
						snapshot_id: next_nonce,
						state_hash,
						state_change_id: batch.stid,
						last_processed_blk: state_info.last_block.saturated_into(),
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

		state_info.snapshot_id = batch.snapshot_id; // Store the processed nonce
		Self::store_state_info(state_info, &mut state)?;
		state.commit();
		store_trie_root(*state.root());
		Ok(())
	}

	fn import_blk(
		blk: T::BlockNumber,
		state: &mut TrieDBMut<LayoutV1<BlakeTwo256>>,
		state_info: &mut StateInfo,
	) -> Result<(), &'static str> {
		log::info!(target:"ocex","Importing block: {:?}",blk);

		if blk <= state_info.last_block.saturated_into() {
			return Err("BlockOutofSequence")
		}

		let messages = Self::ingress_messages(blk);

		for message in messages {
			// We don't care about any other message
			match message {
				IngressMessages::Deposit(main, asset, amt) => add_balance(
					state,
					&Decode::decode(&mut &main.encode()[..])
						.map_err(|_| "account id decode error")?,
					asset,
					amt,
				)?,
				_ => {},
			}
		}

		state_info.last_block = blk.saturated_into();

		Ok(())
	}

	fn trades(
		trades: &Vec<Trade>,
		state: &mut TrieDBMut<LayoutV1<BlakeTwo256>>,
	) -> Result<(), &'static str> {
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
		state: &mut TrieDBMut<LayoutV1<BlakeTwo256>>,
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

	fn process_batch(
		state: &mut TrieDBMut<LayoutV1<BlakeTwo256>>,
		batch: &UserActionBatch<T::AccountId>,
		state_info: &mut StateInfo,
	) -> Result<Vec<Withdrawal<T::AccountId>>, &'static str> {
		if state_info.stid >= batch.stid {
			return Err("Invalid stid")
		}

		let mut withdrawals = Vec::new();
		// Process Ob messages
		for action in &batch.actions {
			match action {
				UserActions::Trade(trades) => Self::trades(trades, state)?,
				UserActions::Withdraw(request) => {
					let withdrawal = Self::withdraw(request, state, batch.stid)?;
					withdrawals.push(withdrawal);
				},
				UserActions::BlockImport(blk) =>
					Self::import_blk((*blk).saturated_into(), state, state_info)?,
				UserActions::Reset => {}, // Not for offchain worker
			}
		}

		Ok(withdrawals)
	}

	fn load_state_info(state: &TrieDBMut<LayoutV1<BlakeTwo256>>) -> StateInfo {
		match state.get(&STATE_INFO) {
			Ok(Some(data)) => StateInfo::decode(&mut &data[..]).unwrap_or_default(),
			Ok(None) => StateInfo::default(),
			Err(_) => StateInfo::default(),
		}
	}

	fn store_state_info(
		state_info: StateInfo,
		state: &mut TrieDBMut<LayoutV1<BlakeTwo256>>,
	) -> Result<(), &'static str> {
		let _ = state
			.insert(&STATE_INFO, &state_info.encode())
			.map_err(|err| map_trie_error(err))?;
		Ok(())
	}
}

pub fn map_trie_error<T, E>(err: Box<TrieError<T, E>>) -> &'static str {
	match *err {
		TrieError::InvalidStateRoot(_) => "Invalid State Root",
		TrieError::IncompleteDatabase(_) => "Incomplete Database",
		TrieError::ValueAtIncompleteKey(_, _) => "ValueAtIncompleteKey",
		TrieError::DecoderError(_, _) => "DecoderError",
		TrieError::InvalidHash(_, _) => "InvalidHash",
	}
}
