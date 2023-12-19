use crate::{
	pallet::{Config, ExpectedLMPConfig, LMPConfig, Pallet},
	FinalizeLMPScore, LMPEpoch,
};
use frame_system::pallet_prelude::BlockNumberFor;
use orderbook_primitives::traits::LiquidityMiningCrowdSourcePallet;
use sp_runtime::SaturatedConversion;

// TODO: Check if have 27 days periodicity  condition for stopping withdrawal
// TODO: will have any unexpected artifact or loophole.
impl<T: Config> Pallet<T> {
	pub(crate) fn should_start_new_epoch(n: BlockNumberFor<T>) -> bool {
		n.saturated_into::<u32>() % 201600u32 == 0 // 28 days in blocks
	}

	/// Starts new liquidity mining epoch
	pub(crate) fn start_new_epoch() {
		let mut current_epoch: u16 = <LMPEpoch<T>>::get();
		if <FinalizeLMPScore<T>>::get().is_none() {
			<FinalizeLMPScore<T>>::put(current_epoch);
		}
		current_epoch = current_epoch.saturating_add(1);
		<LMPEpoch<T>>::put(current_epoch);
		let config = <ExpectedLMPConfig<T>>::get();
		<LMPConfig<T>>::insert(current_epoch, config);
	}

	pub(crate) fn should_stop_accepting_lmp_withdrawals(n: BlockNumberFor<T>) -> bool {
		n.saturated_into::<u32>() % 194400u32 == 0 // 27 days in blocks
	}

	pub(crate) fn stop_accepting_lmp_withdrawals() {
		let current_epoch: u16 = <LMPEpoch<T>>::get();
		T::CrowdSourceLiqudityMining::stop_accepting_lmp_withdrawals(current_epoch)
	}
}
