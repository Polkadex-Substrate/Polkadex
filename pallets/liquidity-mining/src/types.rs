use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use rust_decimal::Decimal;
use scale_info::TypeInfo;

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo)]
pub struct MarketMakerConfig<AccountId> {
	pub pool_id: AccountId,
	pub commission: Decimal,
	pub exit_fee: Decimal,
	pub public_funds_allowed: bool,
	pub name: [u8; 10],
	pub share_id: u128,
	pub force_closed: bool,
}

pub type EpochNumber = u32;
