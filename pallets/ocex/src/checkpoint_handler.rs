use sp_runtime::offchain::storage::StorageValueRef;

const CHECKPOINT_FLAG: [u8; 30] = *b"offchain-ocex::checkpoint_flag";

struct CheckpointHandler;

impl CheckpointHandler {
	fn get_checkpoint_flag() -> bool {
		let s_r = StorageValueRef::persistent(&CHECKPOINT_FLAG);
		match s_r.get::<bool>() {
			Ok(Some(x)) => x,
			Ok(None) => {
				log::trace!(target:"ocex","checkpoint_flag not found");
				false
			},
			Err(_) => {
				log::error!(target:"ocex","Failed to get checkpoint_flag");
				false
			},
		}
	}

	fn set_checkpoint_flag(flag: bool) {
		let s_w = StorageValueRef::persistent(&CHECKPOINT_FLAG);
		s_w.set(&flag);
	}
}
