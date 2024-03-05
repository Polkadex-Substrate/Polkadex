use crate::pallet::IngressMessages;
use crate::{
	pallet::{Config, ExpectedLMPConfig, LMPConfig, Pallet},
	FinalizeLMPScore, LMPEpoch,
};
use frame_system::pallet_prelude::BlockNumberFor;
use orderbook_primitives::traits::LiquidityMiningCrowdSourcePallet;
use sp_runtime::SaturatedConversion;

const EPOCH_LENGTH: u32 = 200u32; // 28 days in blocks //FIXME: Change it back

impl<T: Config> Pallet<T> {
	pub(crate) fn should_start_new_epoch(n: BlockNumberFor<T>) -> bool {
		n.saturated_into::<u32>() % EPOCH_LENGTH == 0
	}

	/// Starts new liquidity mining epoch
	pub fn start_new_epoch(n: BlockNumberFor<T>) {
		if let Some(config) = <ExpectedLMPConfig<T>>::get() {
			let mut current_epoch: u16 = <LMPEpoch<T>>::get();
			//This is to handle the corner case when epoch is 0
			if current_epoch == 0 && !<LMPConfig<T>>::contains_key(current_epoch) {
				<LMPConfig<T>>::insert(current_epoch, config.clone());
			}
			if <FinalizeLMPScore<T>>::get().is_none() {
				<FinalizeLMPScore<T>>::put(current_epoch);
			}
			current_epoch = current_epoch.saturating_add(1);
			<LMPEpoch<T>>::put(current_epoch);
			<LMPConfig<T>>::insert(current_epoch, config);
			// Notify Liquidity Crowd sourcing pallet about new epoch
			T::CrowdSourceLiqudityMining::new_epoch(current_epoch);

			<IngressMessages<T>>::mutate(n, |ingress_messages| {
				ingress_messages.push(orderbook_primitives::ingress::IngressMessages::NewLMPEpoch(
					current_epoch,
				))
			});
		}
	}

	pub(crate) fn should_stop_accepting_lmp_withdrawals(n: BlockNumberFor<T>) -> bool {
		// Triggers 7200 blocks ( or approx 1 day before epoch change)
		n.saturated_into::<u32>().saturating_add(7200) % EPOCH_LENGTH == 0
	}

	pub(crate) fn stop_accepting_lmp_withdrawals() {
		let current_epoch: u16 = <LMPEpoch<T>>::get();
		T::CrowdSourceLiqudityMining::stop_accepting_lmp_withdrawals(current_epoch)
	}
}
