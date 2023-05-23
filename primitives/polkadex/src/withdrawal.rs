use crate::assets::AssetId;
use codec::{Decode, Encode, MaxEncodedLen};
use rust_decimal::Decimal;
use scale_info::TypeInfo;

use crate::AccountId;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Withdrawal<AccountId> {
	pub main_account: AccountId,
	pub amount: Decimal,
	pub asset: AssetId,
	pub fees: Decimal,
	pub stid: u64,
	pub worker_nonce: u64,
}

#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct WithdrawalPayload {
	pub asset_id: AssetId,
	pub amount: Decimal,
	pub user: AccountId,
}

#[derive(Encode, Decode, Debug, Clone, TypeInfo, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Withdrawals {
	pub withdrawals: sp_std::vec::Vec<WithdrawalPayload>,
	pub nonce: u32,
}
