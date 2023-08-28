use parity_scale_codec::{Decode, Encode};
use parking_lot::RwLock;
use sp_core::{offchain::OffchainStorage, Bytes};
use std::sync::Arc;

pub const WORKER_STATUS: [u8; 28] = *b"offchain-ocex::worker_status";

/// Adapter to Access OCEX Offchain Storage
pub struct OffchainStorageAdapter<T: OffchainStorage> {
	storage: Arc<RwLock<T>>,
}

impl<T: OffchainStorage> OffchainStorageAdapter<T> {

	/// Create a new `OffchainStorageAdapter` instance.
	/// # Parameters
	/// * `storage`: Offchain storage
	/// # Returns
	/// * `OffchainStorageAdapter`: A new `OffchainStorageAdapter` instance.
	pub fn new(storage: Arc<RwLock<T>>) -> Self {
		Self { storage }
	}

	/// Get worker status
	/// # Returns
	/// * `bool`: Worker status
	pub fn get_worker_status(&self) -> bool {
		let prefix = sp_offchain::STORAGE_PREFIX;
		let bytes: Option<Bytes> = self.storage.read().get(prefix, &WORKER_STATUS).map(Into::into);
		match bytes {
			Some(encoded_bytes) => {
				let encoded_value = encoded_bytes.to_vec();
				match Decode::decode(&mut &encoded_value[..]) {
					Ok(worker_status) => worker_status,
					Err(_) => false,
				}
			},
			None => false,
		}
	}

	/// Update worker status
	/// # Parameters
	/// * `status`: Worker status
	pub fn update_worker_status(&self, status: bool) {
		let prefix = sp_offchain::STORAGE_PREFIX;
		let encoded_value = Encode::encode(&status);
		self.storage.write().set(prefix, &WORKER_STATUS, &encoded_value);
	}
}
