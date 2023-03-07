#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{dispatch::DispatchError, traits::tokens::Balance as BalanceT};
use num_bigint::{BigUint, ToBigUint};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{traits::Zero, RuntimeDebug};
use sp_std::prelude::*;

#[derive(
	Encode,
	Decode,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	Ord,
	TypeInfo,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Pool<CurrencyId, Balance, BlockNumber> {
	pub base_amount: Balance,
	pub quote_amount: Balance,
	pub base_amount_last: Balance,
	pub quote_amount_last: Balance,
	pub lp_token_id: CurrencyId,
	pub block_timestamp_last: BlockNumber,
	pub price_0_cumulative_last: Balance,
	pub price_1_cumulative_last: Balance,
}

impl<CurrencyId, Balance: BalanceT, BlockNumber: BalanceT> Pool<CurrencyId, Balance, BlockNumber> {
	pub fn new(lp_token_id: CurrencyId) -> Self {
		Self {
			base_amount: Zero::zero(),
			quote_amount: Zero::zero(),
			base_amount_last: Zero::zero(),
			quote_amount_last: Zero::zero(),
			lp_token_id,
			block_timestamp_last: Zero::zero(),
			price_0_cumulative_last: Zero::zero(),
			price_1_cumulative_last: Zero::zero(),
		}
	}

	pub fn is_empty(&self) -> bool {
		self.base_amount.is_zero() && self.quote_amount.is_zero()
	}
}

/// Exported traits from our AMM pallet. These functions are to be used
/// by the router to enable multi route token swaps
pub trait AMM<AccountId, CurrencyId, Balance, BlockNumber> {
	/// Based on the path specified and the available pool balances
	/// this will return the amounts outs when trading the specified
	/// amount in
	fn get_amounts_out(
		amount_in: Balance,
		path: Vec<CurrencyId>,
	) -> Result<Vec<Balance>, DispatchError>;

	/// Based on the path specified and the available pool balances
	/// this will return the amounts in needed to produce the specified
	/// amount out
	fn get_amounts_in(
		amount_out: Balance,
		path: Vec<CurrencyId>,
	) -> Result<Vec<Balance>, DispatchError>;

	/// Handles a "swap" on the AMM side for "who".
	/// This will move the `amount_in` funds to the AMM PalletId,
	/// trade `pair.0` to `pair.1` and return a result with the amount
	/// of currency that was sent back to the user.
	fn swap(
		who: &AccountId,
		pair: (CurrencyId, CurrencyId),
		amount_in: Balance,
	) -> Result<(), DispatchError>;

	/// Iterate keys of asset pair in AMM Pools
	fn get_pools() -> Result<Vec<(CurrencyId, CurrencyId)>, DispatchError>;

	///  Returns pool by lp_asset
	fn get_pool_by_lp_asset(
		asset_id: CurrencyId,
	) -> Option<(CurrencyId, CurrencyId, Pool<CurrencyId, Balance, BlockNumber>)>;

	/// Returns pool by asset pair
	fn get_pool_by_asset_pair(
		pair: (CurrencyId, CurrencyId),
	) -> Option<Pool<CurrencyId, Balance, BlockNumber>>;
}

pub trait ConvertToBigUint {
	fn get_big_uint(&self) -> BigUint;
}

impl ConvertToBigUint for u128 {
	fn get_big_uint(&self) -> BigUint {
		self.to_biguint().unwrap()
	}
}
