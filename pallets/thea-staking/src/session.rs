use frame_support::RuntimeDebug;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::traits::{Get, Saturating, Zero};
use std::collections::BTreeSet;

use crate::{BLSPublicKey, BalanceOf, Config, Network, Pallet, SessionIndex};

/// The amount of exposure (to slashing) than an individual nominator has.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct IndividualExposure<T: Config, AccountId> {
	/// The stash account of the nominator in question.
	pub who: AccountId,
	/// Amount of funds exposed.
	#[codec(compact)]
	pub value: BalanceOf<T>,
	/// Backing candidate
	pub backing: Option<(Network, T::AccountId)>,
	/// Any balance that is becoming free, which may eventually be transferred out of the stash
	/// (assuming it doesn't get slashed first). It is assumed that this will be treated as a first
	/// in, first out queue where the new (higher value) eras get pushed on the back.
	// TODO: Bound this to MaxUnlockChunks
	pub unlocking: Vec<UnlockChunk<T>>,
}

impl<T: Config, AccountId> IndividualExposure<T, AccountId> {
	/// Unbond stake of a nominator
	pub fn unbond(&mut self, mut amount: BalanceOf<T>, session_that_will_unlock: SessionIndex) {
		// If the user entered amount is greater than available bonded funds then take available
		// bond
		if self.value < amount {
			amount = self.value
		}
		self.unlocking
			.push(UnlockChunk { value: amount, era: session_that_will_unlock });
	}

	/// Withdraw the unbonded stake
	pub fn withdraw_unbonded(&mut self, current_session: SessionIndex) -> BalanceOf<T> {
		let available_chunks = self
			.unlocking
			.drain_filter(|chunk| chunk.era <= current_session)
			.collect::<Vec<UnlockChunk<T>>>();
		let mut amount_available_to_withdraw: BalanceOf<T> = Default::default();

		for chunk in available_chunks {
			amount_available_to_withdraw = amount_available_to_withdraw.saturating_add(chunk.value)
		}
		amount_available_to_withdraw
	}
}

/// A snapshot of the stake backing a single relayer in the system.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct Exposure<T: Config, AccountId: PartialEq + Clone + Ord> {
	/// Score of relayer
	pub score: u32,
	/// The total active balance backing this relayer.
	#[codec(compact)]
	pub total: BalanceOf<T>,
	/// BLS public key
	pub bls_pub_key: BLSPublicKey,
	/// The portions of nominators stashes that are exposed.
	pub stakers: BTreeSet<AccountId>,
}

impl<T: Config, AccountId: PartialEq + Clone + Ord> Exposure<T, AccountId> {
	pub fn new(bls_pub_key: BLSPublicKey) -> Self {
		Self { score: 1000, total: Default::default(), bls_pub_key, stakers: Default::default() }
	}
	/// Adds the given stake to own and update the total
	pub fn add_own_stake(&mut self, stake: BalanceOf<T>) {
		self.total = self.total.saturating_add(stake);
	}
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
/// Defines the limits of staking algorithm
pub struct StakingLimits<Balance: Zero> {
	pub mininum_relayer_stake: Balance,
	pub minimum_nominator_stake: Balance,
	pub maximum_nominator_per_relayer: u32,
	pub max_relayers: u32,
}

impl<Balance: Zero> Default for StakingLimits<Balance> {
	fn default() -> Self {
		Self {
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

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
#[derive(
	PartialEq, Eq, Clone, Ord, PartialOrd, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct UnlockChunk<T: Config> {
	/// Amount of funds to be unlocked.
	#[codec(compact)]
	pub(crate) value: BalanceOf<T>,
	/// Era number at which point it'll be unlocked.
	#[codec(compact)]
	era: SessionIndex,
}
