#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
use polkadex_primitives::assets::AssetId;
pub use polkadex_primitives::{AccountId, Balance, BlockNumber, Hash};
#[cfg(feature = "std")]
use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeStruct};
#[cfg(feature = "std")]
use sp_core::crypto::Ss58Codec;
use sp_std::vec::Vec;
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct FundingRoundWithPrimitives<AccountId> {
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_custom_tuple"))]
    pub token_a: AssetId,
    pub creator: AccountId,
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub amount: Balance,
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_custom_tuple"))]
    pub token_b: AssetId,
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub vesting_per_block: Balance,
    pub start_block: BlockNumber,
    pub vote_end_block: BlockNumber,
    pub vesting_end_block : BlockNumber,
    pub project_info_cid: Vec<u8>,
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub min_allocation: Balance,
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub max_allocation: Balance,
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub token_a_priceper_token_b: Balance,
    pub close_round_block: BlockNumber,
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub actual_raise: Balance,
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
    s.parse::<T>()
        .map_err(|_| serde::de::Error::custom("Parse from string failed"))
}


#[cfg(feature = "std")]
fn serialize_as_custom_tuple<S: Serializer>(
    t: &AssetId,
    serializer: S,
) -> Result<S::Ok, S::Error> {

    match t {
        AssetId::POLKADEX => {
            let mut s = serializer.serialize_struct("asset", 1)?;
            let f : Option<u64> = None;
            s.serialize_field("polkadex", &f);
            s.end()
        }
        AssetId::Asset(id) => {
            let mut s = serializer.serialize_struct("asset", 1)?;
            s.serialize_field("asset" , &id.to_string());
            s.end()
        }
    }
}



#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct VoteStat {
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub yes: Balance,
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub no: Balance,
}