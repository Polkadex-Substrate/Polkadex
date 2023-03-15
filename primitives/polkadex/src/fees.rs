use codec::{Decode, Encode};
use rust_decimal::{prelude::Zero, Decimal};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Encode, Decode, PartialEq, Debug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct FeeConfig {
	pub(crate) maker_fraction: Decimal,
	pub(crate) taker_fraction: Decimal,
}

impl Default for FeeConfig {
	fn default() -> Self {
		Self { maker_fraction: Decimal::zero(), taker_fraction: Decimal::zero() }
	}
}
