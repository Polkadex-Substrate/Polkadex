use crate::pallet::{Config, Pallet};
use frame_system::pallet_prelude::BlockNumberFor;

impl<T: Config> Pallet<T> {
	pub(crate) fn should_start_new_session(n: BlockNumberFor<T>) -> bool {
		todo!()
	}

	pub(crate) fn should_start_withdrawals(n: BlockNumberFor<T>) -> bool {
		todo!()
	}

	/// Starts new liquidity mining session
	pub(crate) fn start_new_session(n: BlockNumberFor<T>) {
		todo!()
	}

	/// Starts new liquidity mining session
	pub(crate) fn process_withdrawals(n: BlockNumberFor<T>) {
		todo!()
	}
}
