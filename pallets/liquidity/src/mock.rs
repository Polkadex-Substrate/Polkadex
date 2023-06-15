// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Tests for liquidity pallet

use crate::{pallet as liquidity, LiquidityModifier, *};
use frame_support::{
	pallet_prelude::Weight,
	parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU128, ConstU64, OnTimestampSet},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSigned};
use polkadex_primitives::{AccountId, AssetId, Moment, Signature};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::cell::RefCell;
// use pallet_ocex_lmp;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// For testing the pallet, we construct a mock runtime.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		OCEX: pallet_ocex_lmp::{Pallet, Call, Storage, Event<T>},
		Liquidity: liquidity::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(Weight::from_ref_time(1024));
}
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_balances::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Balance = u128;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type ReserveIdentifier = [u8; 8];
	type HoldIdentifier = ();
	type FreezeIdentifier = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type MaxHolds = ();
	type MaxFreezes = ();
}

thread_local! {
	pub static CAPTURED_MOMENT: RefCell<Option<Moment>> = RefCell::new(None);
}

pub struct MockOnTimestampSet;
impl OnTimestampSet<Moment> for MockOnTimestampSet {
	fn on_timestamp_set(moment: Moment) {
		CAPTURED_MOMENT.with(|x| *x.borrow_mut() = Some(moment));
	}
}

impl pallet_timestamp::Config for Test {
	type Moment = Moment;
	type OnTimestampSet = MockOnTimestampSet;
	type MinimumPeriod = ConstU64<5>;
	type WeightInfo = ();
}

parameter_types! {
	pub const ProxyLimit: u32 = 2;
	pub const OcexPalletId: PalletId = PalletId(*b"OCEX_LMP");
	pub const MsPerDay: u64 = 86_400_000;
}

impl pallet_ocex_lmp::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = OcexPalletId;
	type GovernanceOrigin = EnsureRoot<AccountId>;
	type NativeCurrency = Balances;
	type OtherAssets = Assets;
	type EnclaveOrigin = EnsureRoot<AccountId>;
	type Public = <Signature as sp_runtime::traits::Verify>::Signer;
	type Signature = Signature;
	type MsPerDay = MsPerDay;
	type WeightInfo = pallet_ocex_lmp::weights::WeightInfo<Test>;
}

//defined trait for Session Change
impl<Test> LiquidityModifier for pallet_ocex_lmp::Pallet<Test> {
	type AssetId = AssetId;
	type AccountId = AccountId;

	fn on_deposit(
		_account: Self::AccountId,
		_asset: Self::AssetId,
		_balance: u128,
	) -> DispatchResult {
		Ok(())
	}
	fn on_withdraw(
		_account: Self::AccountId,
		_proxy_account: Self::AccountId,
		_asset: Self::AssetId,
		_balance: u128,
		_do_force_withdraw: bool,
	) -> DispatchResult {
		Ok(())
	}
	fn on_register(_main_account: Self::AccountId, _proxy: Self::AccountId) -> DispatchResult {
		Ok(())
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn set_exchange_state_to_true() -> DispatchResult {
		Ok(())
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn allowlist_and_create_token(_account: Self::AccountId, _token: u128) -> DispatchResult {
		Ok(())
	}
}

parameter_types! {
	pub const AssetDeposit: u128 = 100;
	pub const ApprovalDeposit: u128 = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: u128 = 10;
	pub const MetadataDepositPerByte: u128 = 1;
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type AssetId = u128;
	type Currency = Balances;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = ();
	type AssetIdParameter = parity_scale_codec::Compact<u128>;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type CallbackHandle = ();
	type RemoveItemsLimit = ();
}

parameter_types! {
	pub const LiquidityPalletId: PalletId = PalletId(*b"LIQUIDID");
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = LiquidityPalletId;
	type NativeCurrency = Balances;
	type Public = <Signature as sp_runtime::traits::Verify>::Signer;
	type Signature = Signature;
	type GovernanceOrigin = EnsureRoot<AccountId>;
	type CallOcex = OCEX;
	type WeightInfo = super::weights::WeightInfo<Test>;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}
