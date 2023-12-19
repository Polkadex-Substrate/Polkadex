use crate::{
	pallet::{Config, ExpectedLMPConfig, LMPConfig, Pallet},
	FinalizeLMPScore, LMPEpoch,
};
use frame_system::pallet_prelude::BlockNumberFor;
use orderbook_primitives::traits::LiquidityMiningCrowdSourcePallet;

impl<T: Config> Pallet<T> {
	pub(crate) fn should_start_new_epoch(n: BlockNumberFor<T>) -> bool {
		todo!()
	}

	/// Starts new liquidity mining epoch
	pub(crate) fn start_new_epoch(n: BlockNumberFor<T>) {
		let mut current_epoch: u16 = <LMPEpoch<T>>::get();
		if <FinalizeLMPScore<T>>::get().is_none() {
			<FinalizeLMPScore<T>>::put(current_epoch);
		}
		// TODO: Insert new epoch code here.
		// TODO: Calculate market weightage, total rewards, and other params and initialize a market
		// config for this epoch
		current_epoch = current_epoch.saturating_add(1);
		<LMPEpoch<T>>::put(current_epoch);
		let config = <ExpectedLMPConfig<T>>::get();
		<LMPConfig<T>>::insert(current_epoch, config);
		todo!()
	}

	pub(crate) fn should_stop_accepting_lmp_withdrawals(n: BlockNumberFor<T>) -> bool {
		todo!()
	}

	pub(crate) fn stop_accepting_lmp_withdrawals(n: BlockNumberFor<T>) {
		let current_epoch: u16 = <LMPEpoch<T>>::get();
		T::CrowdSourceLiqudityMining::stop_accepting_lmp_withdrawals(current_epoch)
	}
}
