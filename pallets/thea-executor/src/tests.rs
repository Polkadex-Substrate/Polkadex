// This file is part of Polkadex.

// Copyright (C) 2020-2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

use crate::mock::{new_test_ext, RuntimeOrigin as Origin, Test, *};
use frame_support::{assert_err, assert_noop, assert_ok};
use parity_scale_codec::Encode;
use sp_core::H160;
use sp_runtime::{traits::ConstU32, BoundedVec, TokenError};

use thea_primitives::{parachain::ParachainDeposit, types::Withdraw};
use xcm::{
	latest::{AssetId, Fungibility, Junction, Junctions, MultiAsset, MultiLocation, NetworkId},
	prelude::X1,
};

#[test]
fn test_withdraw_with_wrong_benificiary_length() {
	new_test_ext().execute_with(|| {
		let beneficiary: [u8; 1001] = [1; 1001];
		assert_noop!(
			TheaExecutor::withdraw(
				RuntimeOrigin::signed(1),
				1u128,
				1000u128,
				beneficiary.to_vec(),
				false
			),
			crate::Error::<Test>::BeneficiaryTooLong
		);
	})
}

#[test]
fn transfer_native_asset() {
	new_test_ext().execute_with(|| {
		let asset_id = 1000;
		let para_id = 2040;
		let multi_location = MultiLocation {
			parents: 1,
			interior: X1(Junction::AccountId32 { network: NetworkId::Any, id: [1; 32] }),
		};
		let asset_location =
			MultiLocation { parents: 1, interior: Junctions::X1(Junction::Parachain(para_id)) };
		let asset = MultiAsset {
			id: AssetId::Concrete(asset_location),
			fun: 10_000_000_000_000u128.into(),
		};
		assert_ok!(TheaExecutor::set_withdrawal_fee(RuntimeOrigin::root(), 1, 0));
		// Mint Asset to Alice
		assert_ok!(Balances::set_balance(RuntimeOrigin::root(), 1, 1_000_000_000_000_000_000, 0));
		assert_ok!(TheaExecutor::withdraw(
			RuntimeOrigin::signed(1),
			asset_id.clone(),
			10_000_000_000_000u128,
			multi_location.encode().to_vec(),
			false
		));
		let pending_withdrawal = <PendingWithdrawals<Test>>::get(1);
		let payload = ParachainWithdraw::get_parachain_withdraw(asset, multi_location.clone());
		let approved_withdraw = Withdraw {
			asset_id,
			amount: 10_000_000_000_000u128,
			network: 1,
			beneficiary: multi_location.encode().to_vec(),
			payload: payload.encode(),
			index: 0,
		};
		assert_eq!(pending_withdrawal.to_vec().pop().unwrap(), approved_withdraw);
	})
}

#[test]
fn claim_deposit_pass_with_proper_inputs() {
	new_test_ext().execute_with(|| {
		let mut ad = vec![];
		const NETWORK: u8 = 0;
		const LEN: usize = 5;
		// asset build stuff
		const ASSET_ADDRESS: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
		let asset = ASSET_ADDRESS.parse::<H160>().unwrap();
		let asset_addr = asset.to_fixed_bytes();
		let mut derived_asset_id = vec![];
		derived_asset_id.push(NETWORK);
		derived_asset_id.push(LEN as u8);
		let id: BoundedVec<u8, ConstU32<1000>> = asset_addr.to_vec().try_into().unwrap();
		derived_asset_id.extend(&id[0..LEN]);
		let asset_id = AssetHandler::get_asset_id(derived_asset_id);
		// create asset
		assert_ok!(AssetHandler::allowlist_token(Origin::signed(1), asset));
		assert_ok!(AssetHandler::create_thea_asset(Origin::signed(1), NETWORK, LEN as u8, id));
		// check no deposit error
		assert_err!(
			TheaExecutor::claim_deposit(Origin::signed(1), 100),
			Error::<Test>::NoApprovedDeposit
		);
		// generate max number of deposits
		for i in 1..101u128 {
			let d = ApprovedDeposit {
				recipient: 1 as u64,
				network_id: NETWORK,
				amount: i.saturating_add(100_000).saturating_mul(100_000),
				asset_id,
				tx_hash: [i as u8; 32].into(),
			};
			ad.push(d);
		}
		let ad: BoundedVec<
			ApprovedDeposit<<Test as frame_system::Config>::AccountId>,
			ConstU32<100>,
		> = ad.try_into().unwrap();
		<ApprovedDeposits<Test>>::insert(1, ad);
		// check it can't create on execute_deposit with wrong account
		assert_err!(TheaExecutor::claim_deposit(Origin::signed(1), 100), TokenError::CannotCreate);
		// call extrinsic and check it passes
		assert_ok!(Balances::set_balance(Origin::root(), 1, 1_000_000_000_000, 0));
		assert_ok!(TheaExecutor::claim_deposit(Origin::signed(1), 100));
		assert!(<ApprovedDeposits<Test>>::get(1).is_none());
	});
}

#[test]
fn test_deposit_with_valid_args_returns_ok() {
	new_test_ext().execute_with(|| {
		let multi_asset = MultiAsset {
			id: AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here }),
			fun: Fungibility::Fungible(10_u128),
		};
		let multi_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 { network: NetworkId::Any, id: [1; 32] }),
		};
		let new_payload = ParachainDeposit {
			recipient: multi_location,
			asset_and_amount: multi_asset,
			deposit_nonce: 1,
			transaction_hash: sp_core::H256::zero(),
			network_id: 1,
		};
		let asset_id = AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here });
		assert_ok!(asset_handler::pallet::Pallet::<Test>::create_parachain_asset(
			RuntimeOrigin::signed(1),
			Box::from(asset_id)
		));
		assert_ok!(TheaExecutor::do_deposit(1, new_payload.encode()));
	})
}

#[test]
fn test_deposit_with_zero_amount_returns_err() {
	new_test_ext().execute_with(|| {
		let multi_asset = MultiAsset {
			id: AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here }),
			fun: Fungibility::Fungible(0_u128),
		};
		let multi_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 { network: NetworkId::Any, id: [1; 32] }),
		};
		let new_payload = ParachainDeposit {
			recipient: multi_location,
			asset_and_amount: multi_asset,
			deposit_nonce: 1,
			transaction_hash: sp_core::H256::zero(),
			network_id: 1,
		};
		let asset_id = AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here });
		assert_ok!(asset_handler::pallet::Pallet::<Test>::create_parachain_asset(
			RuntimeOrigin::signed(1),
			Box::from(asset_id)
		));
		assert_noop!(
			TheaExecutor::do_deposit(1, new_payload.encode()),
			crate::Error::<Test>::AmountCannotBeZero
		);
	})
}

#[test]
fn test_deposit_with_asset_not_regsutered_returns_err() {
	new_test_ext().execute_with(|| {
		let multi_asset = MultiAsset {
			id: AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here }),
			fun: Fungibility::Fungible(10_u128),
		};
		let multi_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 { network: NetworkId::Any, id: [1; 32] }),
		};
		let new_payload = ParachainDeposit {
			recipient: multi_location,
			asset_and_amount: multi_asset,
			deposit_nonce: 1,
			transaction_hash: sp_core::H256::zero(),
			network_id: 1,
		};
		assert_noop!(
			TheaExecutor::do_deposit(1, new_payload.encode()),
			Error::<Test>::AssetNotRegistered
		);
	})
}
