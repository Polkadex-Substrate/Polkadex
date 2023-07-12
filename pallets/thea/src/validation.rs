use crate::{Config, Pallet};

impl<T: Config> Pallet<T> {
	pub fn run_thea_validation(blk: T::BlockNumber) -> Result<(), &'static str> {
		todo!()
	}
}
