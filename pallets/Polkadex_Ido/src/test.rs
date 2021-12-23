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
use crate::mock::*;
use frame_support::assert_noop;
use polkadex_primitives::assets::AssetId;
use sp_core::H160;
use sp_runtime::traits::Hash;

#[test]
fn test_register_investor() {
    // Register new account
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(
            PolkadexIdo::register_investor(Origin::signed(ALICE.clone())),
            Ok(())
        );
        assert_noop!(
            PolkadexIdo::register_investor(Origin::signed(ALICE.clone())),
            Error::<Test>::InvestorAlreadyRegistered
        );
    });
}

#[test]
fn test_attest_investor() {
    let bob: u64 = 5;
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            PolkadexIdo::attest_investor(Origin::signed(bob), ALICE.clone(), KYCStatus::Tier1),
            Error::<Test>::InvestorDoesNotExist
        );
        assert_eq!(
            PolkadexIdo::register_investor(Origin::signed(ALICE.clone())),
            Ok(())
        );
        assert_eq!(
            PolkadexIdo::attest_investor(Origin::signed(bob), ALICE.clone(), KYCStatus::Tier1),
            Ok(())
        );
    });
}

#[test]
fn test_register_round() {
    let balance: Balance = 100;
    let open_block_number = 0;
    let closing_block_number = 5;
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(
            PolkadexIdo::register_round(
                Origin::signed(ALICE.clone()),
                AssetId::CHAINSAFE(H160::from_low_u64_be(24)),
                balance,
                AssetId::POLKADEX,
                balance,
                open_block_number,
                balance,
                balance,
                balance,
                balance,
                closing_block_number
            ),
            Ok(())
        );
    });
}

#[test]
fn test_whitelist_investor() {
    let balance: Balance = 100;
    let investor_address: u64 = 4;
    let open_block_number = 0;
    let closing_block_number = 5;
    let round_id = create_hash_data(&1u32);
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            PolkadexIdo::whitelist_investor(
                Origin::signed(ALICE.clone()),
                round_id,
                investor_address,
                balance
            ),
            Error::<Test>::FundingRoundDoesNotExist
        );

        assert_eq!(
            PolkadexIdo::register_round(
                Origin::signed(ALICE.clone()),
                AssetId::CHAINSAFE(H160::from_low_u64_be(24)),
                balance,
                AssetId::POLKADEX,
                balance,
                open_block_number,
                balance,
                balance,
                balance,
                balance,
                closing_block_number
            ),
            Ok(())
        );

        let round_id = <InfoProjectTeam<Test>>::get(ALICE.clone());
        assert_noop!(
            PolkadexIdo::whitelist_investor(
                Origin::signed(investor_address),
                round_id,
                ALICE.clone(),
                balance
            ),
            Error::<Test>::NotACreater
        );

        assert_noop!(
            PolkadexIdo::whitelist_investor(
                Origin::signed(ALICE.clone()),
                round_id,
                investor_address,
                balance
            ),
            Error::<Test>::InvestorDoesNotExist
        );

        assert_eq!(
            PolkadexIdo::register_investor(Origin::signed(investor_address)),
            Ok(())
        );

        assert_eq!(
            PolkadexIdo::whitelist_investor(
                Origin::signed(ALICE.clone()),
                round_id,
                investor_address,
                balance
            ),
            Ok(())
        );
    });
}

#[test]
fn test_participate_in_round() {
    let balance: Balance = 100;
    let investor_address: u64 = 4;
    let round_id = create_hash_data(&1u32);
    let open_block_number = 0;
    let closing_block_number = 5;
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            PolkadexIdo::participate_in_round(Origin::signed(ALICE.clone()), round_id, balance),
            Error::<Test>::NotWhiteListed
        );

        assert_eq!(
            PolkadexIdo::register_round(
                Origin::signed(ALICE.clone()),
                AssetId::CHAINSAFE(H160::from_low_u64_be(24)),
                balance,
                AssetId::POLKADEX,
                balance,
                open_block_number,
                balance,
                balance,
                balance,
                balance,
                closing_block_number
            ),
            Ok(())
        );

        let round_id = <InfoProjectTeam<Test>>::get(ALICE.clone());

        assert_eq!(
            PolkadexIdo::register_investor(Origin::signed(investor_address)),
            Ok(())
        );

        assert_eq!(
            PolkadexIdo::whitelist_investor(
                Origin::signed(ALICE.clone()),
                round_id,
                investor_address,
                balance
            ),
            Ok(())
        );

        // Investment under minimum amount should return an error
        assert_noop!(
            PolkadexIdo::participate_in_round(
                Origin::signed(investor_address),
                round_id,
                balance - 1
            ),
            Error::<Test>::NotAValidAmount
        );

        assert_eq!(
            PolkadexIdo::participate_in_round(Origin::signed(investor_address), round_id, balance),
            Ok(())
        );

        //Checks if the user can participate in a round more than once
        assert_noop!(
            PolkadexIdo::participate_in_round(Origin::signed(investor_address), round_id, balance),
            Error::<Test>::InvestorAlreadyParticipated
        );

        // Check if FundingRound was successfully updated after investment
        let round_info: FundingRound<Test> = <InfoFundingRound<Test>>::get(round_id);
        assert_eq!(round_info.actual_raise == balance, true);
    });
}

#[test]
fn test_claim_tokens() {
    let balance: Balance = 100;
    let investor_address: u64 = 4;
    let open_block_number = 0;
    let closing_block_number = 5;
    let round_id = create_hash_data(&1u32);
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            PolkadexIdo::claim_tokens(Origin::signed(investor_address), round_id,),
            Error::<Test>::InvestorDoesNotExist
        );

        assert_eq!(
            PolkadexIdo::register_investor(Origin::signed(investor_address)),
            Ok(())
        );

        assert_noop!(
            PolkadexIdo::claim_tokens(Origin::signed(investor_address), round_id,),
            Error::<Test>::FundingRoundDoesNotExist
        );

        frame_system::Pallet::<Test>::set_block_number(closing_block_number);

        assert_eq!(
            PolkadexIdo::register_round(
                Origin::signed(ALICE.clone()),
                AssetId::CHAINSAFE(H160::from_low_u64_be(24)),
                balance,
                AssetId::POLKADEX,
                balance,
                open_block_number,
                balance,
                balance,
                balance,
                balance,
                closing_block_number
            ),
            Ok(())
        );

        let round_id = <InfoProjectTeam<Test>>::get(ALICE.clone());

        assert_eq!(
            PolkadexIdo::claim_tokens(Origin::signed(investor_address), round_id,),
            Ok(())
        );

        assert_eq!(
            LastClaimBlockInfo::<Test>::contains_key(round_id, investor_address),
            true
        );

        assert_eq!(
            InfoClaimAmount::<Test>::contains_key(round_id, investor_address),
            true
        );
    });
}

#[test]
fn test_show_interest_in_round() {
    let balance: Balance = 500;
    let investor_address: u64 = 4;
    let amount: Balance = 200;
    let min_allocation: Balance = 100;
    let max_allocation: Balance = 400;
    let round_id = create_hash_data(&1u32);
    let open_block_number = 0;
    let closing_block_number = 5;
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            PolkadexIdo::show_interest_in_round(Origin::signed(investor_address), round_id, amount),
            Error::<Test>::InvestorDoesNotExist
        );

        assert_eq!(
            PolkadexIdo::register_investor(Origin::signed(investor_address)),
            Ok(())
        );

        assert_noop!(
            PolkadexIdo::show_interest_in_round(Origin::signed(investor_address), round_id, amount),
            Error::<Test>::FundingRoundDoesNotExist
        );

        assert_eq!(
            PolkadexIdo::register_round(
                Origin::signed(ALICE.clone()),
                AssetId::CHAINSAFE(H160::from_low_u64_be(24)),
                balance,
                AssetId::POLKADEX,
                balance,
                open_block_number,
                min_allocation,
                max_allocation,
                balance,
                balance,
                closing_block_number
            ),
            Ok(())
        );

        let round_id = <InfoProjectTeam<Test>>::get(ALICE.clone());
        //Check investing with lower than minimum allocation
        assert_noop!(
            PolkadexIdo::show_interest_in_round(
                Origin::signed(investor_address),
                round_id,
                min_allocation - 1
            ),
            Error::<Test>::NotAValidAmount
        );
        //Check investing with more than max allocation
        assert_noop!(
            PolkadexIdo::show_interest_in_round(
                Origin::signed(investor_address),
                round_id,
                max_allocation + 1
            ),
            Error::<Test>::NotAValidAmount
        );

        assert_eq!(
            PolkadexIdo::show_interest_in_round(Origin::signed(investor_address), round_id, amount),
            Ok(())
        );
    });
}
// Show Interest
// add some investors and have them to show interest to participate
// One investor of lowest amount will be randomly evicted
// verify the most invested was not get evicted
#[test]
fn test_show_interest_in_round_randomized_participants() {
    let balance: Balance = 500;
    let min_allocation: Balance = 100;
    let max_allocation: Balance = 400;
    let open_block_number = 0;
    let closing_block_number = 5;
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(
            PolkadexIdo::register_round(
                Origin::signed(ALICE.clone()),
                AssetId::CHAINSAFE(H160::from_low_u64_be(24)),
                balance,
                AssetId::POLKADEX,
                balance,
                open_block_number,
                min_allocation,
                max_allocation,
                balance,
                balance,
                closing_block_number
            ),
            Ok(())
        );

        let round_id = <InfoProjectTeam<Test>>::get(ALICE.clone());

        let investors: Vec<(u64, Balance)> =
            vec![(4u64, 200), (2u64, 200), (5u64, 200), (6u64, 300)];

        for (investor_address, amount) in investors {
            assert_eq!(
                PolkadexIdo::register_investor(Origin::signed(investor_address)),
                Ok(())
            );
            assert_eq!(
                PolkadexIdo::show_interest_in_round(
                    Origin::signed(investor_address),
                    round_id,
                    amount
                ),
                Ok(())
            );
        }

        let funding_round: FundingRound<Test> = <InfoFundingRound<Test>>::get(round_id);

        let total_investment_amount: Balance =
            InterestedParticipants::<Test>::iter_prefix_values(round_id)
                .fold(0_u128, |sum, amount| sum.saturating_add(amount));
        let investors_count = InterestedParticipants::<Test>::iter_prefix_values(round_id).count();
        // Check if an investor was randomly evicted
        assert_eq!(investors_count <= 3, true);
        assert_eq!(
            InterestedParticipants::<Test>::contains_key(round_id, 6u64),
            true
        );
        // Check if maximum effective investors are selected
        assert_eq!(total_investment_amount >= funding_round.amount, true);
    });
}

#[test]
fn test_withdraw_raise() {
    let balance: Balance = 100;
    let investor_address: u64 = 4;
    let open_block_number = 0;
    let closing_block_number = 5;
    let round_id = create_hash_data(&1u32);
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            PolkadexIdo::withdraw_raise(Origin::signed(ALICE), round_id, investor_address),
            Error::<Test>::InvestorDoesNotExist
        );
        assert_eq!(
            PolkadexIdo::register_investor(Origin::signed(investor_address)),
            Ok(())
        );

        assert_noop!(
            PolkadexIdo::withdraw_raise(Origin::signed(ALICE), round_id, investor_address),
            Error::<Test>::FundingRoundDoesNotExist
        );

        assert_eq!(
            PolkadexIdo::register_round(
                Origin::signed(ALICE),
                AssetId::CHAINSAFE(H160::from_low_u64_be(24)),
                balance,
                AssetId::POLKADEX,
                balance,
                open_block_number,
                balance,
                balance,
                balance,
                balance,
                closing_block_number
            ),
            Ok(())
        );

        let round_id = <InfoProjectTeam<Test>>::get(ALICE);

        assert_noop!(
            PolkadexIdo::withdraw_raise(Origin::signed(3), round_id, investor_address),
            Error::<Test>::NotACreater
        );

        assert_eq!(PolkadexIdo::register_investor(Origin::signed(2)), Ok(()));
        <Test as Config>::Currency::deposit(
            AssetId::CHAINSAFE(H160::from_low_u64_be(24)),
            &4_u64,
            100000,
        );
        assert_eq!(
            PolkadexIdo::register_round(
                Origin::signed(4),
                AssetId::CHAINSAFE(H160::from_low_u64_be(24)),
                balance,
                AssetId::POLKADEX,
                balance,
                open_block_number,
                balance,
                balance,
                balance,
                balance,
                closing_block_number
            ),
            Ok(())
        );


        assert_noop!(
            PolkadexIdo::withdraw_raise(Origin::signed(4), round_id, 2),
            Error::<Test>::NotACreater
        );

        //Test creator withdraw during when fundraising is not close should return error : WithdrawalBlocked
        system::Pallet::<Test>::set_block_number(closing_block_number - 1);
        assert_noop!(
            PolkadexIdo::withdraw_raise(Origin::signed(ALICE), round_id, investor_address),
            Error::<Test>::WithdrawalBlocked
        );

        //Test creator withdraw during when fundraising is closed should be successful
        system::Pallet::<Test>::set_block_number(closing_block_number);
        assert_eq!(
            PolkadexIdo::withdraw_raise(Origin::signed(ALICE), round_id, investor_address),
            Ok(())
        );
    });
}

#[test]
fn test_withdraw_token() {
    let balance: Balance = 100;
    let investor_address: u64 = 4;
    let open_block_number = 0;
    let closing_block_number = 5;
    let round_id = create_hash_data(&1u32);
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            PolkadexIdo::withdraw_token(Origin::signed(ALICE), round_id, investor_address),
            Error::<Test>::InvestorDoesNotExist
        );
        assert_eq!(
            PolkadexIdo::register_investor(Origin::signed(investor_address)),
            Ok(())
        );

        assert_noop!(
            PolkadexIdo::withdraw_token(Origin::signed(ALICE), round_id, investor_address),
            Error::<Test>::FundingRoundDoesNotExist
        );

        assert_eq!(
            PolkadexIdo::register_round(
                Origin::signed(ALICE),
                AssetId::CHAINSAFE(H160::from_low_u64_be(24)),
                balance,
                AssetId::POLKADEX,
                balance,
                open_block_number,
                balance,
                balance,
                balance,
                balance,
                closing_block_number
            ),
            Ok(())
        );

        let round_id = <InfoProjectTeam<Test>>::get(ALICE);

        assert_noop!(
            PolkadexIdo::withdraw_token(Origin::signed(3), round_id, investor_address),
            Error::<Test>::NotACreater
        );

        assert_eq!(PolkadexIdo::register_investor(Origin::signed(2)), Ok(()));
        <Test as Config>::Currency::deposit(
            AssetId::CHAINSAFE(H160::from_low_u64_be(24)),
            &4_u64,
            100000,
        );
        assert_eq!(
            PolkadexIdo::register_round(
                Origin::signed(4),
                AssetId::CHAINSAFE(H160::from_low_u64_be(24)),
                balance,
                AssetId::POLKADEX,
                balance,
                open_block_number,
                balance,
                balance,
                balance,
                balance,
                closing_block_number
            ),
            Ok(())
        );

        assert_noop!(
            PolkadexIdo::withdraw_token(Origin::signed(4), round_id, 2),
            Error::<Test>::NotACreater
        );

        //Test creator withdraw tokens during when fundraising is not close should return error : WithdrawalBlocked
        system::Pallet::<Test>::set_block_number(closing_block_number - 1);
        assert_noop!(
            PolkadexIdo::withdraw_token(Origin::signed(ALICE), round_id, investor_address),
            Error::<Test>::WithdrawalBlocked
        );

        //Test creator withdraw tokens during when fundraising is closed should be successful
        system::Pallet::<Test>::set_block_number(closing_block_number);
        assert_eq!(
            PolkadexIdo::withdraw_token(Origin::signed(ALICE), round_id, investor_address),
            Ok(())
        );
    });
}

fn create_hash_data(data: &u32) -> <mock::Test as frame_system::Config>::Hash {
    data.using_encoded(<Test as frame_system::Config>::Hashing::hash)
}
