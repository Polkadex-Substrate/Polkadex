// This file is part of Polkadex.

// Copyright (C) 2020-2021 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use super::*;

use crate as polkadex_fungible_assets;
use frame_support::{ord_parameter_types, parameter_types};
use frame_system::EnsureSignedBy;
use orml_traits::parameter_type_with_key;
use polkadex_primitives::assets::AssetId;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::convert::From;
use orml_currencies::BasicCurrencyAdapter;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Storage, Event<T>},
        Fungible: polkadex_fungible_assets::{Module, Call, Event<T>},
        Currencies: orml_currencies::{Module, Call, Event<T>},
        OrmlToken: orml_tokens::{Module, Call, Storage, Event<T>},
        PalletBalances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
    }
);

pub type Balance = u128;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 500;
    pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Test {
    type MaxLocks = MaxLocks;
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type Event = ();
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Test>;
    type WeightInfo = ();
}

parameter_types! {
	pub const GetNativeCurrencyId: AssetId = AssetId::POLKADEX;
}

impl orml_currencies::Config for Test {
    type Event = ();
    type MultiCurrency = OrmlToken;
    type NativeCurrency = AdaptedBasicCurrency;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type WeightInfo = ();
}

parameter_types! {
    pub const TresuryAccount: u64 = 9;
}

ord_parameter_types! {
    pub const Six: u64 = 6;
}

impl Config for Test {
    type Event = ();
    type TreasuryAccountId = TresuryAccount;
    type GovernanceOrigin = EnsureSignedBy<Six, u64>;
    type NativeCurrency =AdaptedBasicCurrency;
}

pub type AdaptedBasicCurrency = BasicCurrencyAdapter<Test, PalletBalances, i128, u128>;

parameter_types! {
    pub TreasuryModuleAccount: u64 = 1;
}

parameter_type_with_key! {
    pub ExistentialDeposits: |_currency_id: AssetId| -> Balance {
        Zero::zero()
    };
}

impl orml_tokens::Config for Test {
    type Event = ();
    type Balance = Balance;
    type Amount = i128;
    type CurrencyId = AssetId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type OnDust = orml_tokens::TransferDust<Test, TreasuryModuleAccount>;
}

pub type PolkadexFungibleAssets = Module<Test>;

pub fn new_tester() -> sp_io::TestExternalities {
    let storage = system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    let mut ext: sp_io::TestExternalities = storage.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}