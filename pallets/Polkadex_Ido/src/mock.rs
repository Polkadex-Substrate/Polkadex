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

use crate as polkadex_ido;
use frame_support::PalletId;
use frame_support::{parameter_types, traits::SortedMembers};
use frame_support_test::TestRandomness;
use frame_system::EnsureSignedBy;
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::arithmetic::Zero;
use orml_traits::parameter_type_with_key;
use polkadex_primitives::assets::AssetId;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::convert::From;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Storage, Event<T>},
        PolkadexIdo: polkadex_ido::{Pallet, Call, Event<T>},
        Currencies: orml_currencies::{Pallet, Call, Event<T>},
        OrmlToken: orml_tokens::{Pallet, Call, Storage, Event<T>},
        PalletBalances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
    }
);

pub type Balance = u128;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

type AccountId = u64;
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
    type AccountId = AccountId;
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
    type OnSetCode = ();
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 0;
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
    pub const PolkadexIdoModuleId: PalletId = PalletId(*b"polk/ido");
}

parameter_types! {
    pub const GetIDOPDXAmount: Balance = 100u128;
    pub const GetMaxSupply: Balance = 200u128;
}

pub struct OneToFive;
impl SortedMembers<u64> for OneToFive {
    fn sorted_members() -> Vec<u64> {
        vec![1, 2, 3, 4, 5]
    }
}

impl Config for Test {
    type Event = ();
    type TreasuryAccountId = TresuryAccount;
    type GovernanceOrigin = EnsureSignedBy<OneToFive, u64>;
    type Currency = OrmlToken;
    type NativeCurrencyId = GetNativeCurrencyId;
    type IDOPDXAmount = GetIDOPDXAmount;
    type MaxSupply = GetMaxSupply;
    type Randomness = TestRandomness<Self>;
    type RandomnessSource = TestRandomness<Self>;
    type ModuleId = PolkadexIdoModuleId;
    type WeightIDOInfo = ();
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

pub const ALICE: AccountId = 1;
pub const INITIAL_BALANCE: Balance = 1_000_000;

pub struct ExtBuilder {
    endowed_accounts: Vec<(AccountId, AssetId, Balance)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            endowed_accounts: vec![
                (ALICE, AssetId::POLKADEX, INITIAL_BALANCE),
                (4, AssetId::POLKADEX, INITIAL_BALANCE),
                (2, AssetId::POLKADEX, INITIAL_BALANCE),
                (5, AssetId::POLKADEX, INITIAL_BALANCE),
                (6, AssetId::POLKADEX, INITIAL_BALANCE),
            ],
        }
    }
}

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        pallet_balances::GenesisConfig::<Test> {
            balances: vec![
                (ALICE, INITIAL_BALANCE),
                (4u64, INITIAL_BALANCE),
                (2u64, INITIAL_BALANCE),
                (5u64, INITIAL_BALANCE),
                (6u64, INITIAL_BALANCE),
            ],
        }
        .assimilate_storage(&mut t)
        .unwrap();

        super::GenesisConfig::<Test> {
            endowed_accounts: self.endowed_accounts,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}
