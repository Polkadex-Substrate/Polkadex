use fixed::{types::extra::U32, FixedU128};
#[cfg(feature = "std")]
use ink_storage::traits::StorageLayout;
use ink_storage::traits::{PackedLayout, SpreadLayout};

pub type ExchangeRate = FixedU128<U32>;
pub type Ratio = FixedU128<U32>;


#[derive(scale::Encode, scale::Decode, Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, PackedLayout, SpreadLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum AssetId {
    /// PDEX the native currency of the chain
    POLKADEX,
    /// Generic enumerated assed
    /// Range 0 - 0x00000000FFFFFFFF (2^32)-1 is reserved for protected tokens
    /// the values under 1000 are used for ISO 4217 Numeric Curency codes
    Asset(u64),
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    PartialOrd,
    Ord,
    PackedLayout,
    SpreadLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct TradingPair(AssetId, AssetId);

impl TradingPair {
    pub fn from_currency_ids(
        currency_id_a: AssetId,
        currency_id_b: AssetId,
    ) -> Option<Self> {
        match currency_id_a {
            AssetId::POLKADEX => {
                match currency_id_b {
                    AssetId::POLKADEX => None,
                    AssetId::Asset(_) => Some(TradingPair(currency_id_a, currency_id_b))
                }
            },
            AssetId::Asset(x) => {
                match currency_id_b {
                    AssetId::POLKADEX => Some(TradingPair(currency_id_b, currency_id_a)),
                    AssetId::Asset(y) => {
                        if x != y {
                            if x > y {
                                Some(TradingPair(currency_id_b, currency_id_a))
                            } else {
                                Some(TradingPair(currency_id_a, currency_id_b))
                            }
                        } else {
                            None
                        }
                    }
                }
            }
        }
    }

    pub fn first(&self) -> AssetId {
        self.0
    }

    pub fn second(&self) -> AssetId {
        self.1
    }
}
