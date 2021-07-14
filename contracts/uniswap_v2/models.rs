#[cfg(feature = "std")]
use ink_storage::traits::StorageLayout;
use ink_storage::traits::{PackedLayout, SpreadLayout};
use sp_core::{H160};
use sp_runtime::{FixedU128};

pub type Balance = <ink_env::DefaultEnvironment as ink_env::Environment>::Balance;

pub type ExchangeRate = FixedU128;

// #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode, PartialOrd, Ord, PackedLayout, SpreadLayout)]
// #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub type TokenAddress = H160;

#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode, PartialOrd, Ord, PackedLayout, SpreadLayout)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct TradingPair(TokenAddress, TokenAddress);

impl TradingPair {
	pub fn from_currency_ids(currency_id_a: TokenAddress, currency_id_b: TokenAddress) -> Option<Self> {
		if currency_id_a != currency_id_b {
			if currency_id_a > currency_id_b {
				Some(TradingPair(currency_id_b, currency_id_a))
			} else {
				Some(TradingPair(currency_id_a, currency_id_b))
			}
		} else {
			None
		}
	}

	pub fn first(&self) -> TokenAddress {
		self.0
	}

	pub fn second(&self) -> TokenAddress {
		self.1
	}
}