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
use crate::{
	mock::{new_test_ext, Test, *},
	pallet::{ApprovedDeposit, *},
	util::*,
};
use frame_support::{
	assert_err, assert_noop, assert_ok, error::BadOrigin, traits::fungibles::Mutate,
};
use parity_scale_codec::Encode;
use sp_core::{H160, H256};
use sp_runtime::{traits::ConstU32, BoundedVec, TokenError};
use thea_primitives::{
	parachain_primitives::{AssetType, ParachainAsset, ParachainDeposit, ParachainWithdraw},
	ApprovedWithdraw, TokenType,
};
use xcm::{
	latest::{AssetId, Fungibility, Junction, Junctions, MultiAsset, MultiLocation, NetworkId},
	prelude::X1,
};

#[test]
fn test_approve_deposit_with_bad_origin_should_fail() {
	new_test_ext().execute_with(|| {
		let sig = [1; 96];
		let mut bit_map_2 = 0_u128;
		bit_map_2 = set_kth_bit(bit_map_2, 0);
		bit_map_2 = set_kth_bit(bit_map_2, 1);

		let asset_id = AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here });
		let wrong_payload = [1; 32];
		assert_ok!(asset_handler::pallet::Pallet::<Test>::create_parachain_asset(
			Origin::signed(1),
			Box::from(asset_id)
		));
		assert_noop!(
			Thea::approve_deposit(
				Origin::none(),
				bit_map_2,
				sig.into(),
				TokenType::Fungible(1_u8),
				wrong_payload.to_vec()
			),
			BadOrigin
		);
	})
}

#[test]
fn test_approve_deposit_with_right_inputs_return_ok() {
	new_test_ext().execute_with(|| {
		register_bls_public_keys();
		let multi_asset = MultiAsset {
			id: AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here }),
			fun: Fungibility::Fungible(10_u128),
		};
		let multi_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 {
				network: NetworkId::Any,
				id: create_account_id().encode().try_into().unwrap(),
			}),
		};
		let new_payload = ParachainDeposit {
			recipient: multi_location,
			asset_and_amount: multi_asset,
			deposit_nonce: 1,
			transaction_hash: sp_core::H256::zero(),
			network_id: 1,
		};
		let sig = sign_payload(new_payload.encode());

		let mut bit_map_2 = 0_u128;
		bit_map_2 = set_kth_bit(bit_map_2, 0);
		bit_map_2 = set_kth_bit(bit_map_2, 1);

		let asset_id = AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here });
		assert_ok!(asset_handler::pallet::Pallet::<Test>::create_parachain_asset(
			RuntimeOrigin::signed(1),
			Box::from(asset_id)
		));
		assert_ok!(Thea::approve_deposit(
			RuntimeOrigin::signed(1),
			bit_map_2,
			sig.into(),
			TokenType::Fungible(1_u8),
			new_payload.encode()
		));
	})
}

#[test]
fn test_approve_deposit_returns_failed_to_decode() {
	new_test_ext().execute_with(|| {
		let sig = [1; 96];
		let mut bit_map_2 = 0_u128;
		bit_map_2 = set_kth_bit(bit_map_2, 0);
		bit_map_2 = set_kth_bit(bit_map_2, 1);

		let asset_id = AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here });
		let wrong_payload = [1; 32];
		assert_ok!(asset_handler::pallet::Pallet::<Test>::create_parachain_asset(
			RuntimeOrigin::signed(1),
			Box::from(asset_id)
		));
		assert_noop!(
			Thea::approve_deposit(
				RuntimeOrigin::signed(1),
				bit_map_2,
				sig.into(),
				TokenType::Fungible(1_u8),
				wrong_payload.to_vec()
			),
			Error::<Test>::FailedToDecode
		);
	})
}

#[test]
fn test_approve_deposits_with_wrong_multi_asset_returns_failed_to_handle_parachain_deposit() {
	new_test_ext().execute_with(|| {
		register_bls_public_keys();
		let multi_asset =
			MultiAsset { id: AssetId::Abstract(vec![1; 10]), fun: Fungibility::Fungible(10_u128) };
		let multi_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 {
				network: NetworkId::Any,
				id: create_account_id().encode().try_into().unwrap(),
			}),
		};
		let new_payload = ParachainDeposit {
			recipient: multi_location,
			asset_and_amount: multi_asset,
			deposit_nonce: 1,
			transaction_hash: sp_core::H256::zero(),
			network_id: 1,
		};
		let sig = sign_payload(new_payload.encode());

		let mut bit_map_2 = 0_u128;
		bit_map_2 = set_kth_bit(bit_map_2, 0);
		bit_map_2 = set_kth_bit(bit_map_2, 1);

		let asset_id = AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here });
		assert_ok!(asset_handler::pallet::Pallet::<Test>::create_parachain_asset(
			RuntimeOrigin::signed(1),
			Box::from(asset_id)
		));
		assert_noop!(
			Thea::approve_deposit(
				RuntimeOrigin::signed(1),
				bit_map_2,
				sig.into(),
				TokenType::Fungible(1_u8),
				new_payload.encode()
			),
			Error::<Test>::FailedToHandleParachainDeposit
		);
	})
}

#[test]
fn test_approve_deposits_with_wrong_signature_returns_bls_signature_verification_failed() {
	new_test_ext().execute_with(|| {
		let multi_asset = MultiAsset {
			id: AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here }),
			fun: Fungibility::Fungible(10_u128),
		};
		let multi_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 {
				network: NetworkId::Any,
				id: create_account_id().encode().try_into().unwrap(),
			}),
		};
		let new_payload = ParachainDeposit {
			recipient: multi_location,
			asset_and_amount: multi_asset,
			deposit_nonce: 1,
			transaction_hash: sp_core::H256::zero(),
			network_id: 1,
		};
		let wrong_sig = [1; 96];

		let mut bit_map_2 = 0_u128;
		bit_map_2 = set_kth_bit(bit_map_2, 0);
		bit_map_2 = set_kth_bit(bit_map_2, 1);

		let asset_id = AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here });
		assert_ok!(asset_handler::pallet::Pallet::<Test>::create_parachain_asset(
			RuntimeOrigin::signed(1),
			Box::from(asset_id)
		));
		assert_noop!(
			Thea::approve_deposit(
				RuntimeOrigin::signed(1),
				bit_map_2,
				wrong_sig.into(),
				TokenType::Fungible(1_u8),
				new_payload.encode()
			),
			Error::<Test>::BLSSignatureVerificationFailed
		);
	})
}

#[test]
fn test_approve_deposit_with_zero_amount_return_amount_cannot_be_zero() {
	new_test_ext().execute_with(|| {
		register_bls_public_keys();
		let multi_asset = MultiAsset {
			id: AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here }),
			fun: Fungibility::Fungible(0_u128),
		};
		let multi_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 {
				network: NetworkId::Any,
				id: create_account_id().encode().try_into().unwrap(),
			}),
		};
		let new_payload = ParachainDeposit {
			recipient: multi_location,
			asset_and_amount: multi_asset,
			deposit_nonce: 1,
			transaction_hash: sp_core::H256::zero(),
			network_id: 1,
		};
		let sig = sign_payload(new_payload.encode());

		let mut bit_map_2 = 0_u128;
		bit_map_2 = set_kth_bit(bit_map_2, 0);
		bit_map_2 = set_kth_bit(bit_map_2, 1);

		let asset_id = AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here });
		assert_ok!(asset_handler::pallet::Pallet::<Test>::create_parachain_asset(
			RuntimeOrigin::signed(1),
			Box::from(asset_id)
		));
		assert_noop!(
			Thea::approve_deposit(
				RuntimeOrigin::signed(1),
				bit_map_2,
				sig.into(),
				TokenType::Fungible(1_u8),
				new_payload.encode()
			),
			Error::<Test>::AmountCannotBeZero
		);
	})
}

#[test]
fn test_approve_deposit_with_wrong_nonce_return_deposit_nonce_error() {
	new_test_ext().execute_with(|| {
		register_bls_public_keys();
		let multi_asset = MultiAsset {
			id: AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here }),
			fun: Fungibility::Fungible(1000_u128),
		};
		let multi_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 {
				network: NetworkId::Any,
				id: create_account_id().encode().try_into().unwrap(),
			}),
		};
		let new_payload = ParachainDeposit {
			recipient: multi_location,
			asset_and_amount: multi_asset,
			deposit_nonce: 10,
			transaction_hash: sp_core::H256::zero(),
			network_id: 1,
		};
		let sig = sign_payload(new_payload.encode());

		let mut bit_map_2 = 0_u128;
		bit_map_2 = set_kth_bit(bit_map_2, 0);
		bit_map_2 = set_kth_bit(bit_map_2, 1);

		let asset_id = AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here });
		assert_ok!(asset_handler::pallet::Pallet::<Test>::create_parachain_asset(
			RuntimeOrigin::signed(1),
			Box::from(asset_id)
		));
		assert_noop!(
			Thea::approve_deposit(
				RuntimeOrigin::signed(1),
				bit_map_2,
				sig.into(),
				TokenType::Fungible(1_u8),
				new_payload.encode()
			),
			Error::<Test>::DepositNonceError
		);
	})
}

#[test]
fn test_approve_deposit_with_unregistered_asset_return_asset_not_registered() {
	new_test_ext().execute_with(|| {
		register_bls_public_keys();
		let multi_asset = MultiAsset {
			id: AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here }),
			fun: Fungibility::Fungible(1000_u128),
		};
		let multi_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 {
				network: NetworkId::Any,
				id: create_account_id().encode().try_into().unwrap(),
			}),
		};
		let new_payload = ParachainDeposit {
			recipient: multi_location,
			asset_and_amount: multi_asset,
			deposit_nonce: 1,
			transaction_hash: sp_core::H256::zero(),
			network_id: 1,
		};
		let sig = sign_payload(new_payload.encode());

		let mut bit_map_2 = 0_u128;
		bit_map_2 = set_kth_bit(bit_map_2, 0);
		bit_map_2 = set_kth_bit(bit_map_2, 1);

		assert_noop!(
			Thea::approve_deposit(
				RuntimeOrigin::signed(1),
				bit_map_2,
				sig.into(),
				TokenType::Fungible(1_u8),
				new_payload.encode()
			),
			Error::<Test>::AssetNotRegistered
		);
	})
}

#[test]
fn test_withdraw_with_pay_remaining_false_returns_ok() {
	new_test_ext().execute_with(|| {
		let asset_id = AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here });
		let multi_asset = MultiAsset {
			id: AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here }),
			fun: Fungibility::Fungible(1000_u128),
		};
		let multi_location = MultiLocation {
			parents: 1,
			interior: X1(Junction::AccountId32 { network: NetworkId::Any, id: [1; 32] }),
		};
		assert_ok!(asset_handler::pallet::Pallet::<Test>::create_parachain_asset(
			RuntimeOrigin::signed(1),
			Box::from(asset_id.clone())
		));
		assert_ok!(Thea::set_withdrawal_fee(RuntimeOrigin::root(), 1, 0));
		let beneficiary: [u8; 32] = [1; 32];
		// Mint Asset to Alice
		assert_ok!(pallet_balances::pallet::Pallet::<Test>::set_balance(
			RuntimeOrigin::root(),
			1,
			1_000_000_000_000,
			0
		));
		assert_ok!(pallet_assets::pallet::Pallet::<Test>::mint_into(
			generate_asset_id(asset_id.clone()),
			&1,
			1_000_000_000_000
		));
		let beneficiary: MultiLocation = MultiLocation::new(
			1,
			Junctions::X1(Junction::AccountId32 { network: NetworkId::Any, id: beneficiary }),
		);
		assert_ok!(Thea::withdraw(
			RuntimeOrigin::signed(1),
			generate_asset_id(asset_id.clone()),
			1000u128,
			beneficiary.encode().to_vec(),
			false
		));
		let pending_withdrawal = <PendingWithdrawals<Test>>::get(1);
		let payload = ParachainWithdraw::get_parachain_withdraw(multi_asset, multi_location);
		let approved_withdraw = ApprovedWithdraw {
			asset_id: generate_asset_id(asset_id),
			amount: 1000,
			network: 1,
			beneficiary: beneficiary.encode(),
			payload: payload.encode(),
			index: 0,
		};
		assert_eq!(pending_withdrawal.to_vec().pop().unwrap(), approved_withdraw);
	})
}

#[test]
fn test_withdraw_returns_proper_errors_and_ok() {
	new_test_ext().execute_with(|| {
		let asset_id = AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here });
		let multi_asset = MultiAsset {
			id: AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here }),
			fun: Fungibility::Fungible(1000_u128),
		};
		let multi_location = MultiLocation {
			parents: 1,
			interior: X1(Junction::AccountId32 { network: NetworkId::Any, id: [1; 32] }),
		};
		assert_ok!(asset_handler::pallet::Pallet::<Test>::create_parachain_asset(
			RuntimeOrigin::signed(1),
			Box::from(asset_id.clone())
		));
		assert_ok!(Thea::set_withdrawal_fee(RuntimeOrigin::root(), 1, 0));
		let beneficiary: [u8; 32] = [1; 32];
		let beneficiary: MultiLocation = MultiLocation::new(
			1,
			Junctions::X1(Junction::AccountId32 { network: NetworkId::Any, id: beneficiary }),
		);
		// Mint Asset to Alice
		assert_ok!(Balances::set_balance(Origin::root(), 1, 1_000_000_000_000, 0));
		assert_ok!(Assets::mint_into(generate_asset_id(asset_id.clone()), &1, 1_000_000_000_000));
		// bad origin test
		assert_err!(
			Thea::withdraw(
				Origin::none(),
				generate_asset_id(asset_id.clone()),
				1000u128,
				beneficiary.encode().to_vec(),
				false
			),
			BadOrigin
		);
		// network key rotation happening test
		<TheaKeyRotation<Test>>::insert(1, true);
		assert_err!(
			Thea::withdraw(
				Origin::signed(1),
				generate_asset_id(asset_id.clone()),
				1000u128,
				beneficiary.encode().to_vec(),
				false
			),
			Error::<Test>::TheaKeyRotationInPlace
		);
		<TheaKeyRotation<Test>>::insert(1, false);
		// withdrawal not allowed test
		let old_withdrawals = Thea::pending_withdrawals(1);
		let mut withdrawals = vec![];
		let payload = ParachainWithdraw::get_parachain_withdraw(multi_asset, multi_location);
		for _ in 1..=10 {
			withdrawals.push(ApprovedWithdraw {
				asset_id: generate_asset_id(asset_id.clone()),
				amount: 1000,
				network: 1,
				beneficiary: vec![1; 32],
				payload: payload.encode(),
				index: 0,
			});
		}
		let withdrawals: BoundedVec<ApprovedWithdraw, ConstU32<10>> =
			withdrawals.try_into().unwrap();
		<PendingWithdrawals<Test>>::insert(1, withdrawals);
		assert_err!(
			Thea::withdraw(
				Origin::signed(1),
				generate_asset_id(asset_id.clone()),
				1000u128,
				beneficiary.encode().to_vec(),
				false
			),
			Error::<Test>::WithdrawalNotAllowed
		);
		<PendingWithdrawals<Test>>::insert(1, old_withdrawals);
		// good orogin test
		assert_ok!(Thea::withdraw(
			RuntimeOrigin::signed(1),
			generate_asset_id(asset_id.clone()),
			1000u128,
			beneficiary.encode().to_vec(),
			false
		));
		let pending_withdrawal = <PendingWithdrawals<Test>>::get(1);
		let approved_withdraw = ApprovedWithdraw {
			asset_id: generate_asset_id(asset_id),
			amount: 1000,
			network: 1,
			beneficiary: beneficiary.encode(),
			payload: payload.encode(),
			index: 0,
		};
		assert_eq!(pending_withdrawal.to_vec().pop().unwrap(), approved_withdraw);
	})
}

#[test]
fn test_withdraw_with_wrong_benificiary_length() {
	new_test_ext().execute_with(|| {
		let beneficiary: [u8; 1001] = [1; 1001];
		assert_noop!(
			Thea::withdraw(RuntimeOrigin::signed(1), 1u128, 1000u128, beneficiary.to_vec(), false),
			Error::<Test>::BeneficiaryTooLong
		);
	})
}

#[test]
fn test_withdraw_with_wrong_asset_id_returns_unable_find_network_for_asset_id() {
	new_test_ext().execute_with(|| {
		let beneficiary: [u8; 32] = [1; 32];
		assert_noop!(
			Thea::withdraw(RuntimeOrigin::signed(1), 1u128, 1000u128, beneficiary.to_vec(), false),
			Error::<Test>::UnableFindNetworkForAssetId
		);
	})
}

#[test]
fn test_withdraw_with_no_fee_config() {
	new_test_ext().execute_with(|| {
		let beneficiary: [u8; 32] = [1; 32];
		let beneficiary: MultiLocation = MultiLocation::new(
			1,
			Junctions::X1(Junction::AccountId32 { network: NetworkId::Any, id: beneficiary }),
		);
		let asset_id = AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here });
		assert_ok!(asset_handler::pallet::Pallet::<Test>::create_parachain_asset(
			RuntimeOrigin::signed(1),
			Box::from(asset_id.clone())
		));
		assert_noop!(
			Thea::withdraw(
				RuntimeOrigin::signed(1),
				generate_asset_id(asset_id),
				1000u128,
				beneficiary.encode().to_vec(),
				false
			),
			Error::<Test>::WithdrawalFeeConfigNotFound
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
		assert_ok!(Thea::set_withdrawal_fee(Origin::root(), 1, 0));
		// Mint Asset to Alice
		assert_ok!(Balances::set_balance(Origin::root(), 1, 1_000_000_000_000_000_000, 0));
		assert_ok!(Thea::withdraw(
			RuntimeOrigin::signed(1),
			asset_id.clone(),
			10_000_000_000_000u128,
			multi_location.encode().to_vec(),
			false
		));
		let pending_withdrawal = <PendingWithdrawals<Test>>::get(1);
		let payload = ParachainWithdraw::get_parachain_withdraw(asset, multi_location.clone());
		let approved_withdraw = ApprovedWithdraw {
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
fn test_withdrawal_returns_ok() {
	new_test_ext().execute_with(|| {
		let asset_id = AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here });
		assert_ok!(asset_handler::pallet::Pallet::<Test>::create_parachain_asset(
			RuntimeOrigin::signed(1),
			Box::from(asset_id.clone())
		));
		let asset_id = generate_asset_id(asset_id);
		assert_ok!(pallet_balances::pallet::Pallet::<Test>::set_balance(
			RuntimeOrigin::root(),
			1,
			1_000_000_000_000,
			0
		));
		assert_ok!(pallet_assets::pallet::Pallet::<Test>::mint_into(
			asset_id,
			&1,
			1000000000000u128
		));
		assert_ok!(Thea::set_withdrawal_fee(Origin::root(), 1, 0));
		let beneficiary: [u8; 32] = [1; 32];
		let beneficiary: MultiLocation = MultiLocation::new(
			1,
			Junctions::X1(Junction::AccountId32 { network: NetworkId::Any, id: beneficiary }),
		);
		assert_ok!(Thea::do_withdraw(
			1,
			asset_id,
			1000000000u128,
			beneficiary.encode().to_vec(),
			false
		));
	})
}

pub fn generate_asset_id(asset_id: AssetId) -> u128 {
	if let AssetId::Concrete(ml) = asset_id {
		let parachain_asset = ParachainAsset { location: ml, asset_type: AssetType::Fungible };
		let asset_identifier =
			BoundedVec::<u8, ConstU32<1000>>::try_from(parachain_asset.encode()).unwrap();
		let identifier_length = asset_identifier.len();
		let mut derived_asset_id: Vec<u8> = vec![];
		derived_asset_id.push(1u8);
		derived_asset_id.push(identifier_length as u8);
		derived_asset_id.extend(&asset_identifier.to_vec());
		let derived_asset_id_hash = &sp_io::hashing::keccak_256(derived_asset_id.as_ref())[0..16];
		let mut temp = [0u8; 16];
		temp.copy_from_slice(derived_asset_id_hash);
		u128::from_le_bytes(temp)
	} else {
		0
	}
}

#[test]
fn router_method_should_error_on_non_fungibles() {
	new_test_ext().execute_with(|| {
		assert!(Thea::router(TokenType::NonFungible(1), vec!()).is_err());
		assert!(Thea::router(TokenType::Generic(0), vec!()).is_err());
		assert!(Thea::router(TokenType::Fungible(3), vec!()).is_err());
	});
}

const ASSET_ADDRESS: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";

#[test]
fn claim_deposit_pass_with_proper_inputs() {
	new_test_ext().execute_with(|| {
		let mut ad = vec![];
		const NETWORK: u8 = 0;
		const LEN: usize = 5;
		// asset build stuff
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
		assert_err!(Thea::claim_deposit(Origin::signed(1), 100), Error::<Test>::NoApprovedDeposit);
		// generate max number of deposits
		for i in 1..101u128 {
			let d = ApprovedDeposit {
				recipient: 1 as u64,
				network_id: NETWORK,
				deposit_nonce: i as u32,
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
		assert_err!(Thea::claim_deposit(Origin::signed(1), 100), TokenError::CannotCreate);
		// call extrinsic and check it passes
		assert_ok!(Balances::set_balance(Origin::root(), 1, 1_000_000_000_000, 0));
		assert_ok!(Thea::claim_deposit(Origin::signed(1), 100));
		assert!(<ApprovedDeposits<Test>>::get(1).is_none());
	});
}

#[test]
fn batch_withdrawal_complete_works() {
	new_test_ext().execute_with(|| {
		// create
		const PK: [u8; 64] = [1u8; 64];
		const MAP: u128 = 7;
		const NETWORK: u8 = 1;
		let mut awd = vec![];
		let hash = H256::zero();
		for i in 1..11 {
			awd.push(ApprovedWithdraw {
				asset_id: i,
				amount: i as u128,
				network: NETWORK,
				beneficiary: vec![i as u8],
				payload: vec![i as u8],
				index: i as u32,
			});
		}
		let awd: BoundedVec<ApprovedWithdraw, ConstU32<10>> = awd.try_into().unwrap();
		<ReadyWithdrawls<Test>>::insert(NETWORK, 1, awd);
		let secret_keys = create_three_bls_keys();
		let public_keys = create_bls_public_keys(secret_keys.clone());
		<RelayersBLSKeyVector<Test>>::insert(NETWORK, public_keys);
		<TheaPublicKey<Test>>::insert(NETWORK, PK);
		<WithdrawalNonces<Test>>::insert(NETWORK, 7);
		let nonce = <WithdrawalNonces<Test>>::get(NETWORK);
		let payload = (hash, NETWORK, nonce).encode();
		let signature = sign_payload_with_keys(payload, secret_keys);
		// check errors
		assert!(!Thea::ready_withdrawals(NETWORK, 1).is_empty());
		assert_err!(
			Thea::batch_withdrawal_complete(Origin::signed(1), 1, NETWORK, hash, MAP, signature),
			Error::<Test>::WithdrawalNonceIncorrect
		);
		assert_err!(
			Thea::batch_withdrawal_complete(
				Origin::signed(1),
				nonce,
				NETWORK,
				hash,
				MAP,
				[9u8; 96]
			),
			Error::<Test>::BLSSignatureVerificationFailed
		);
		// check ok
		assert_ok!(Thea::batch_withdrawal_complete(
			Origin::signed(1),
			nonce,
			NETWORK,
			hash,
			MAP,
			signature
		));
	});
}

#[test]
fn test_withdrawal_fee_origins() {
	new_test_ext().execute_with(|| {
		assert_err!(Thea::set_withdrawal_fee(Origin::none(), 1, 1u128), BadOrigin);
		assert_err!(Thea::set_withdrawal_fee(Origin::signed(1), 1, 1u128), BadOrigin);
		assert_ok!(Thea::set_withdrawal_fee(Origin::root(), 1, 1u128));
	});
}

#[test]
fn test_thea_key_rotation() {
	new_test_ext().execute_with(|| {
		// test call fails as expected
		assert_err!(
			Thea::thea_key_rotation_complete(
				Origin::signed(1),
				1,
				H256::default(),
				1u128,
				[1u8; 96]
			),
			Error::<Test>::BLSSignatureVerificationFailed
		);
		let secret_keys = create_three_bls_keys();
		let public_keys = create_bls_public_keys(secret_keys.clone());
		<RelayersBLSKeyVector<Test>>::insert(1, public_keys.clone());
		<QueuedTheaPublicKey<Test>>::insert(1, [1_u8; 64]);
		let payload = (H256::zero(), 1_u8).encode();
		let signature = sign_payload_with_keys(payload.clone(), secret_keys.clone());
		// test call
		// BitMap is 7 because all relayers are signing the payload
		assert_ok!(Thea::thea_key_rotation_complete(
			Origin::signed(1),
			1,
			H256::zero(),
			7,
			signature
		));
	});
}

#[test]
fn test_set_thea_key_complete() {
	new_test_ext().execute_with(|| {
		let secret_keys = create_three_bls_keys();
		let public_keys = create_bls_public_keys(secret_keys.clone());
		let payload = [1_u8; 64].encode();
		let signature = sign_payload_with_keys(payload, secret_keys.clone());
		const PK: [u8; 64] = [1u8; 64];
		const MAP: u128 = 7;
		<RelayersBLSKeyVector<Test>>::insert(1, public_keys.clone());
		<TheaPublicKey<Test>>::insert(1, PK);
		<QueuedTheaPublicKey<Test>>::insert(1, PK);
		// test call no key fails as expected
		assert_err!(
			Thea::set_thea_key_complete(Origin::signed(1), 1, PK, MAP, signature),
			Error::<Test>::QueuedTheaPublicKeyNotFound
		);
		<TheaPublicKey<Test>>::insert(1, [2u8; 64]);
		// test call fails with bad signature
		assert_err!(
			Thea::set_thea_key_complete(Origin::signed(1), 1, PK, MAP, [9u8; 96]),
			Error::<Test>::BLSSignatureVerificationFailed
		);
		// test call
		assert_ok!(Thea::set_thea_key_complete(Origin::signed(1), 1, PK, MAP, signature));
	});
}

#[test]
fn test_thea_queued_queued_public_key() {
	new_test_ext().execute_with(|| {
		let secret_keys = create_three_bls_keys();
		let public_keys = create_bls_public_keys(secret_keys.clone());
		<RelayersBLSKeyVector<Test>>::insert(1, public_keys.clone());
		<QueuedTheaPublicKey<Test>>::insert(1, [1_u8; 64]);

		assert_ok!(Balances::set_balance(Origin::root(), 1, 1_000_000_000_000, 0));
		assert_ok!(Balances::set_balance(Origin::root(), 2, 1_000_000_000_000, 0));
		assert_ok!(Balances::set_balance(Origin::root(), 3, 1_000_000_000_000, 0));

		assert_ok!(TheaStaking::add_candidate(Origin::signed(1), 1, public_keys[0]));
		assert_ok!(TheaStaking::add_candidate(Origin::signed(2), 1, public_keys[1]));
		assert_ok!(TheaStaking::add_candidate(Origin::signed(3), 1, public_keys[2]));

		assert_ok!(TheaStaking::add_network(Origin::root(), 1));

		TheaStaking::rotate_session();
		TheaStaking::rotate_session();

		// Register Candidates on Thea Staking
		let payload = [1_u8; 64].encode();
		let signature = sign_payload_with_keys(payload, secret_keys.clone());
		assert_ok!(Thea::thea_queued_queued_public_key(
			Origin::signed(1),
			1,
			[1u8; 64],
			7,
			signature
		));
	});
}

#[test]
fn test_thea_relayers_reset_rotation() {
	new_test_ext().execute_with(|| {
		assert_ok!(Thea::thea_relayers_reset_rotation(Origin::root(), 1));
	});
}

// hooks tests
#[test]
fn test_on_initialize() {
	new_test_ext().execute_with(|| {
		let msg = <IngressMessages<Test>>::get();
		assert!(msg.is_empty());
	});
}
