use parity_scale_codec::Encode;
use parking_lot::RwLock;
use sp_core::offchain::OffchainStorage;
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

	/// Acquire offchain lock
	/// # Parameters
	/// * `tries`: Number of tries to acquire lock
	/// # Returns
	/// * `bool`: True if lock is acquired else false
	pub fn acquire_offchain_lock(&self, tries: u8) -> bool {
		let prefix = sp_offchain::STORAGE_PREFIX;
		let old_value = Encode::encode(&false);
		let new_value = Encode::encode(&true);
		for _ in 0..tries {
			if self.storage.write().compare_and_set(
				prefix,
				&WORKER_STATUS,
				Some(&old_value),
				&new_value,
			) {
				return true
			}
			// Wait for 1 sec
			std::thread::sleep(std::time::Duration::from_secs(1));
		}
		false
	}
}

impl<T: OffchainStorage> Drop for OffchainStorageAdapter<T> {
	fn drop(&mut self) {
		let prefix = sp_offchain::STORAGE_PREFIX;
		let encoded_value = Encode::encode(&false);
		self.storage.write().set(prefix, &WORKER_STATUS, &encoded_value);
	}
}
