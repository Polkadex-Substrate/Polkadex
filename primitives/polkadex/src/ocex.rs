use crate::assets::AssetId;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::Get, BoundedVec};
use rust_decimal::Decimal;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_std::collections::btree_map::BTreeMap;

use crate::{fees::FeeConfig, withdrawal::Withdrawal, WithdrawalLimit};

#[derive(Clone, Encode, Decode, TypeInfo, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AccountInfo<Account, ProxyLimit: Get<u32>> {
	pub main_account: Account,
	pub proxies: BoundedVec<Account, ProxyLimit>,
	pub balances: BTreeMap<AssetId, (Decimal, Decimal)>,
	/// Trading Fee config
	pub fee_config: FeeConfig,
}
impl<Account: PartialEq, ProxyLimit: Get<u32>> AccountInfo<Account, ProxyLimit> {
	pub fn maker_fee_fraction(&self) -> Decimal {
		self.fee_config.maker_fraction.clone()
	}
	pub fn taker_fee_fraction(&self) -> Decimal {
		self.fee_config.taker_fraction.clone()
	}
}

impl<Account: PartialEq, ProxyLimit: Get<u32>> AccountInfo<Account, ProxyLimit> {
	pub fn new(main_account_id: Account) -> AccountInfo<Account, ProxyLimit> {
		let proxies = BoundedVec::default();
		AccountInfo {
			main_account: main_account_id,
			proxies,
			balances: BTreeMap::new(),
			fee_config: Default::default(),
		}
	}

	// Adds a new proxy account
	pub fn add_proxy(&mut self, proxy: Account) -> Result<(), ()> {
		self.proxies.try_push(proxy)
	}

	// Removes a proxy account
	pub fn remove_proxy(&mut self, proxy: &Account) {
		self.proxies.retain(|item| item != proxy);
	}
}

#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct OCEXConfig<AccountId> {
	pub enclave_id: AccountId,
}

#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TradingPairConfig {
	pub base_asset: AssetId,
	pub quote_asset: AssetId,
	pub min_price: Decimal,
	pub max_price: Decimal,
	pub price_tick_size: Decimal,
	pub min_qty: Decimal,
	pub max_qty: Decimal,
	pub qty_step_size: Decimal,
	pub operational_status: bool, //will be true if the trading pair is enabled on the orderbook.
	pub base_asset_precision: u8,
	pub quote_asset_precision: u8,
}

#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum OnChainEvents<AccountId> {
	OrderBookWithdrawalClaimed(u32, AccountId, BoundedVec<Withdrawal<AccountId>, WithdrawalLimit>),
	GetStorage(Pallet, StorageItem, u32),
}

#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Pallet {
	OCEX,
}

#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum StorageItem {
	Withdrawal,
}
