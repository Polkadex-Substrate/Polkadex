use crate::{
	pallet::{Accounts, SnapshotNonce, UserActionsBatches, ValidatorSetId},
	snapshot::AccountsMap,
	Call, Config, Error, Pallet,
};
use frame_system::offchain::{SendUnsignedTransaction, SignMessage, Signer, SubmitTransaction};
use orderbook_primitives::{types::UserActions, SnapshotSummary};
use parity_scale_codec::Encode;
use sp_application_crypto::RuntimeAppPublic;
use sp_core::H256;
use sp_runtime::offchain::storage::StorageValueRef;
use sp_std::vec::Vec;

pub const WORKER_STATUS: [u8; 28] = *b"offchain-ocex::worker_status";
const ACCOUNTS: [u8; 23] = *b"offchain-ocex::accounts";

impl<T: Config> Pallet<T> {
	pub fn run_on_chain_validation(block_num: T::BlockNumber) -> Result<(), &'static str> {
		// Check if we are a validator
		if !sp_io::offchain::is_validator() {
			// This is not a validator
			return Ok(())
		}

		let local_keys = T::AuthorityId::all();
		let authorities = Self::validator_set().validators;
		let mut available_keys = authorities
			.into_iter()
			.enumerate()
			.filter_map(move |(index, authority)| {
				local_keys
					.binary_search(&authority)
					.ok()
					.map(|location| local_keys[location].clone())
			})
			.collect::<Vec<T::AuthorityId>>();
		available_keys.sort();

		if available_keys.is_empty() {
			return Err("No active keys available")
		}

		// Check if another worker is already running or not
		let s_info = StorageValueRef::persistent(&WORKER_STATUS);
		match s_info.get::<bool>().map_err(|err| "Unable to load worker status")? {
			Some(true) => {
				// Another worker is online, so exit
				return Ok(())
			},
			None => {},
			Some(false) => {},
		}
		s_info.set(&true.encode()); // Set WORKER_STATUS to true
							// Check the next ObMessages to process
		let next_nonce = <SnapshotNonce<T>>::get().saturating_add(1);
		// Load the next ObMessages
		let actions = <UserActionsBatches<T>>::get(next_nonce);
		if actions.is_empty() {
			return Ok(())
		}
		// Load the trie to memory
		let s_info = StorageValueRef::persistent(&ACCOUNTS);
		let accounts =
			match s_info.get::<AccountsMap>().map_err(|err| "Unable to get accounts map")? {
				None => AccountsMap::default(),
				Some(acounts) => acounts,
			};

		let mut withdrawals = Vec::new();
		// Process Ob messages
		for action in actions {
			match action {
				UserActions::Trade(trades) => {},
				UserActions::Withdraw(request) => {},
				UserActions::BlockImport(blk) => {},
			}
		}
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
					worker_nonce: 0,
					state_change_id: 0,
					last_processed_blk: 0,
					withdrawals,
					public: key.clone(),
				};

				let signature = key.sign(&summary.encode()).ok_or("Private key not found")?;

				let call = Call::submit_snapshot { summary, signature };
				SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
					.map_err(|_| "Error sending unsigned txn")?;
			},
		}

		Ok(())
	}
}
