use parity_scale_codec::{Encode, Decode, HasCompact};
use scale_info::TypeInfo;
use sp_runtime::traits::{Get, Saturating, Zero};
use crate::{BalanceOf, Config, Pallet};
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
pub struct Exposure<AccountId: PartialEq + Clone, Balance: HasCompact + Saturating + Copy> {
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

impl<AccountId: PartialEq + Clone, Balance: Default + HasCompact + Saturating + Copy> Default for Exposure<AccountId, Balance> {
    fn default() -> Self {
        Self { score: 1000, total: Default::default(), own: Default::default(), others: vec![] }
    }
}

impl<AccountId: PartialEq + Clone, Balance: Default + HasCompact + Saturating + Copy> Exposure<AccountId, Balance> {
    /// Adds the given stake to own and update the total
    pub fn add_own_stake(&mut self, stake: Balance){
        self.own = self.own.saturating_add(stake);
        self.total = self.total.saturating_add(stake);
    }

    /// Nominate a candidate
    pub fn nominate(&mut self, nominator: &AccountId, amount: Balance) {
        for nominator_exposure in self.others.iter_mut() {
            if &nominator_exposure.who == nominator {
                nominator_exposure.value = nominator_exposure.value.saturating_add(amount);
                self.total = self.total.saturating_add(amount);
            }
            return;
        }
        // it's a new nominator so we add to list
        self.others.push(IndividualExposure{ who: nominator.clone(), value: amount });
        self.total = self.total.saturating_add(amount);
    }

    /// Remove nominator
    pub fn remove_nominator(&mut self, nominator: &AccountId, nominator_index: u32) -> Balance {
        let exposure = self.others.remove(nominator_index as usize);
        if &exposure.who != nominator {
            self.others.push(exposure);
            return Balance::default()
        }
        self.total = self.total.saturating_sub(exposure.value);
        return exposure.value
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
/// Defines the limits of staking algorithm
pub struct StakingLimits<Balance: Zero> {
    pub mininum_relayer_stake: Balance,
    pub minimum_nominator_stake: Balance,
    pub maximum_nominator_per_relayer: u32,
    pub max_relayers: u32
}

impl<Balance: Zero> Default for StakingLimits<Balance>{
    fn default() -> Self {
        Self{
            mininum_relayer_stake: Balance::zero(),
            minimum_nominator_stake: Balance::zero(),
            maximum_nominator_per_relayer: 100,
            max_relayers: 100,
        }
    }
}

impl<T: Config> Pallet<T> {
    // Add public immutables and private mutables.
    pub fn should_end_session(current_block: T::BlockNumber) -> bool {
        (current_block % T::SessionLength::get()).is_zero()
    }
}
