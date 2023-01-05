use parity_scale_codec::{Encode, Decode, HasCompact};
use scale_info::TypeInfo;
use sp_runtime::traits::{Get, Zero};
use crate::{Config, Pallet, SessionIndex};
use frame_support::RuntimeDebug;

/// The amount of exposure (to slashing) than an individual nominator has.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct IndividualExposure<AccountId, Balance: HasCompact> {
    /// The stash account of the nominator in question.
    pub who: AccountId,
    /// Amount of funds exposed.
    #[codec(compact)]
    pub value: Balance,
}

/// A snapshot of the stake backing a single relayer in the system.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Exposure<AccountId, Balance: HasCompact> {
    /// Score of relayer
    pub score: u32,
    /// The total balance backing this relayer.
    #[codec(compact)]
    pub total: Balance,
    /// The relayer's own stash that is exposed.
    #[codec(compact)]
    pub own: Balance,
    /// The portions of nominators stashes that are exposed.
    pub others: Vec<IndividualExposure<AccountId, Balance>>,
}

impl<AccountId, Balance: Default + HasCompact> Default for Exposure<AccountId, Balance> {
    fn default() -> Self {
        Self { score: 1000, total: Default::default(), own: Default::default(), others: vec![] }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct RelayerSet< AuthorityId, Balance>{
    session_index: SessionIndex,
    relayers: Vec<(AuthorityId,Balance)>,
}

impl<T: Config> Pallet<T> {
    // Add public immutables and private mutables.
    pub fn should_end_session(current_block: T::BlockNumber) -> bool {
        (current_block % T::SessionLength::get()).is_zero()
    }
}
