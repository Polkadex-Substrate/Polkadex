use crate::pallet::{Config, Pallet};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::Saturating;

impl<T: Config> Pallet<T> {
    pub(crate) fn should_start_new_epoch(n: BlockNumberFor<T>) -> bool {
        todo!()
    }

    /// Starts new liquidity mining epoch
    pub(crate) fn start_new_epoch(n: BlockNumberFor<T>) {
        let mut current_epoch: u32 = <LMPEpoch<T>>::get();
        // TODO: Insert new epoch code here.
        current_epoch = current_epoch.saturating_add(1);
        <LMPEpoch<T>>::set(current_epoch);
        todo!()
    }

}