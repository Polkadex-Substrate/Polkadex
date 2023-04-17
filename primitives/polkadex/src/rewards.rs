use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// A structure that represents the rewards information associated with an account.
#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct RewardsInfoByAccount<Balance: Default> {
	/// The total amount of rewards that have been claimed by the account.
	pub claimed: Balance,

	/// The total amount of rewards that are unclaimed by the account but have
	/// been earned by participating in crowd loan
	/// provision).
	pub unclaimed: Balance,

	/// The total amount of rewards that are claimable by the account, meaning
	/// the rewards are currently available for the account to claim.
	pub claimable: Balance,
}

