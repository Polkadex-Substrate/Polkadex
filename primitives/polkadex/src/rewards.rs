use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};


#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct RewardsInfoByAccount<Balance> {
    pub claimed: Balance,
    pub unclaimed: Balance,
    pub claimable: Balance
}
