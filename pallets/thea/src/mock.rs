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

use crate::*;
use frame_support::{parameter_types, traits::AsEnsureOriginWithArg, PalletId};
use frame_system as system;
use frame_system::{EnsureRoot, EnsureSigned};
use polkadex_primitives::AssetId;
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage, Permill,
};
use crate::pallet as thea;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		Thea: thea,
		TheaExecutor: thea_executor,
		AssetConversion: pallet_asset_conversion
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type Nonce = u32;
	type Block = Block;
}

parameter_types! {
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
	pub const ExistentialDeposit: u32 = 50;
}

impl pallet_balances::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Test>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = [u8; 8];
	type FreezeIdentifier = ();
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type MaxHolds = MaxLocks;
	type MaxFreezes = ();
}

parameter_types! {
	pub const LockPeriod: u64 = 201600;
	pub const MaxRelayers: u32 = 3;
}

parameter_types! {
	pub const AssetDeposit: Balance = 100;
	pub const ApprovalDeposit: Balance = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 10;
	pub const MetadataDepositPerByte: Balance = 1;
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type RemoveItemsLimit = ();
	type AssetId = u128;
	type AssetIdParameter = parity_scale_codec::Compact<u128>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<u64>>;
	type ForceOrigin = EnsureRoot<u64>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const MaxAuthorities: u32 = 200;
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type TheaId = crate::ecdsa::AuthorityId;
	type Signature = crate::ecdsa::AuthoritySignature;
	type MaxAuthorities = MaxAuthorities;
	type Executor = TheaExecutor;
	type Currency = Balances;
	type GovernanceOrigin = EnsureRoot<u64>;
	type WeightInfo = crate::weights::WeightInfo<Test>;
}

frame_support::ord_parameter_types! {
	pub const AssetConversionOrigin: u32 = 1;
}

parameter_types! {
	pub const AssetConversionPalletId: PalletId = PalletId(*b"py/ascon");
	pub AllowMultiAssetPools: bool = true;
	pub const PoolSetupFee: Balance = 1000000000000; // should be more or equal to the existential deposit
	pub const MintMinLiquidity: Balance = 100;  // 100 is good enough when the main currency has 10-12 decimals.
	pub const LiquidityWithdrawalFee: Permill = Permill::from_percent(0);  // should be non-zero if AllowMultiAssetPools is true, otherwise can be zero.
}
impl pallet_asset_conversion::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type Balance = <Self as pallet_balances::Config>::Balance;
	type AssetBalance = u128;
	type HigherPrecisionBalance = u128;
	type AssetId = u128;
	type MultiAssetId = polkadex_primitives::AssetId;
	type MultiAssetIdConverter = polkadex_primitives::AssetIdConverter;
	type PoolAssetId = u128;
	type Assets = Assets;
	type PoolAssets = Assets;
	type LPFee = ConstU32<3>; // means 0.3%
	type PoolSetupFee = PoolSetupFee;
	type PoolSetupFeeReceiver = AssetConversionOrigin;
	type LiquidityWithdrawalFee = LiquidityWithdrawalFee;
	type MintMinLiquidity = MintMinLiquidity;
	type MaxSwapPathLength = ConstU32<4>;
	type PalletId = AssetConversionPalletId;
	type AllowMultiAssetPools = AllowMultiAssetPools;
	type WeightInfo = pallet_asset_conversion::weights::SubstrateWeight<Test>;
	// #[cfg(feature = "runtime-benchmarks")]
	// type BenchmarkHelper = AssetU128;
}

parameter_types! {
	pub const TheaPalletId: PalletId = PalletId(*b"th/accnt");
	pub const WithdrawalSize: u32 = 10;
	pub const PolkadexAssetId: u128 = 0;
	pub const ParaId: u32 = 2040;
}

impl thea_executor::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type Assets = Assets;
	type AssetId = u128;
	type AssetCreateUpdateOrigin = EnsureRoot<Self::AccountId>;
	type Executor = Thea;
	type NativeAssetId = PolkadexAssetId;
	type TheaPalletId = TheaPalletId;
	type WithdrawalSize = WithdrawalSize;
	type ParaId = ParaId;
	type WeightInfo = thea_executor::weights::WeightInfo<Test>;
	type Swap = AssetConversion;
	type MultiAssetIdAdapter = AssetId;
	type AssetBalanceAdapter = u128;
	type GovernanceOrigin = EnsureRoot<Self::AccountId>;
	type ExistentialDeposit = ExistentialDeposit;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	t.into()
}
