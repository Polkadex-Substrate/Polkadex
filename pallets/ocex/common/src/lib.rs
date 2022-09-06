#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
use polkadex_primitives::assets::AssetId;
pub use polkadex_primitives::{AccountId, Balance, BlockNumber, Hash};
#[cfg(feature = "std")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct WithdrawalWithPrimitives<AccountId> {
	pub main_account: AccountId,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub amount: Balance,
	pub asset: StringAssetId,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub enum StringAssetId {
	POLKADEX,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	Asset(u128),
}

impl From<AssetId> for StringAssetId {
	fn from(asset: AssetId) -> Self {
		match asset {
			AssetId::polkadex => StringAssetId::POLKADEX,
			AssetId::asset(num) => StringAssetId::Asset(num),
		}
	}
}

#[cfg(feature = "std")]
fn serialize_as_string<S: Serializer, T: std::fmt::Display>(
	t: &T,
	serializer: S,
) -> Result<S::Ok, S::Error> {
	serializer.serialize_str(&t.to_string())
}

#[cfg(feature = "std")]
fn deserialize_from_string<'de, D: Deserializer<'de>, T: std::str::FromStr>(
	deserializer: D,
) -> Result<T, D::Error> {
	let s = String::deserialize(deserializer)?;
	s.parse::<T>().map_err(|_| serde::de::Error::custom("Parse from string failed"))
}
