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
};
use blst::min_sig::*;
use frame_support::{
	assert_err, assert_noop, assert_ok, error::BadOrigin, traits::fungibles::Mutate,
};
use parity_scale_codec::Encode;
use sp_core::{crypto::AccountId32, H160, H256};
use sp_keystore::{testing::KeyStore, SyncCryptoStore};
use sp_runtime::{traits::ConstU32, BoundedVec, TokenError};
use thea_primitives::{
	parachain_primitives::{AssetType, ParachainAsset, ParachainDeposit, ParachainWithdraw},
	ApprovedWithdraw, BLSPublicKey, TokenType,
};
use thea_staking::QueuedRelayers;
use xcm::{
	latest::{AssetId, Fungibility, Junction, Junctions, MultiAsset, MultiLocation, NetworkId},
	prelude::X1,
};

pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"ocex");
pub const DST: &[u8; 43] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";
pub const RELAYER_1_BLS_PUBLIC_KEY: [u8; 192] = [
	24, 53, 160, 192, 96, 48, 213, 168, 95, 99, 144, 223, 91, 12, 43, 210, 19, 171, 233, 211, 231,
	95, 200, 248, 240, 79, 172, 104, 178, 210, 77, 211, 58, 221, 165, 57, 182, 25, 75, 59, 186,
	160, 85, 120, 234, 203, 225, 140, 22, 68, 127, 246, 6, 245, 228, 203, 232, 155, 56, 88, 183,
	145, 122, 242, 186, 2, 224, 214, 248, 111, 18, 18, 103, 78, 63, 197, 52, 197, 83, 202, 79, 157,
	110, 97, 192, 128, 26, 226, 32, 210, 59, 18, 194, 91, 157, 102, 12, 235, 187, 24, 41, 197, 12,
	167, 158, 200, 194, 247, 233, 129, 23, 85, 78, 154, 142, 7, 68, 148, 160, 43, 254, 76, 235, 95,
	76, 85, 189, 147, 206, 251, 97, 229, 59, 154, 74, 153, 136, 155, 35, 76, 66, 220, 246, 162, 18,
	159, 243, 223, 177, 184, 150, 155, 140, 147, 93, 249, 175, 131, 143, 40, 110, 48, 89, 248, 34,
	49, 22, 190, 248, 161, 22, 184, 185, 254, 197, 91, 160, 153, 215, 34, 69, 213, 97, 40, 21, 18,
	100, 234, 46, 217, 16, 251,
];
pub const RELAYER_2_BLS_PUBLIC_KEY: [u8; 192] = [
	3, 159, 39, 235, 29, 144, 85, 28, 109, 251, 34, 191, 172, 222, 31, 46, 217, 242, 98, 156, 43,
	195, 80, 220, 170, 58, 138, 231, 251, 222, 178, 12, 157, 170, 166, 107, 228, 39, 209, 143, 123,
	250, 3, 230, 57, 20, 111, 241, 7, 70, 84, 203, 51, 56, 171, 10, 115, 10, 191, 200, 111, 44, 71,
	217, 218, 230, 217, 92, 158, 236, 98, 196, 10, 126, 13, 143, 235, 207, 149, 57, 228, 26, 187,
	169, 39, 107, 156, 68, 184, 116, 125, 96, 53, 163, 209, 117, 12, 162, 94, 175, 159, 75, 14, 55,
	77, 214, 56, 37, 163, 212, 254, 127, 81, 65, 203, 102, 154, 211, 214, 1, 35, 143, 51, 49, 213,
	167, 27, 81, 215, 93, 183, 40, 98, 97, 246, 185, 18, 72, 181, 97, 169, 24, 253, 230, 16, 166,
	139, 111, 199, 52, 110, 245, 13, 38, 212, 85, 114, 135, 144, 198, 192, 221, 224, 107, 197, 24,
	148, 33, 203, 140, 170, 84, 78, 14, 72, 195, 197, 28, 44, 161, 140, 39, 113, 64, 102, 48, 113,
	147, 248, 127, 198, 54,
];
pub const RELAYER_3_BLS_PUBLIC_KEY: [u8; 192] = [
	16, 123, 35, 130, 63, 94, 61, 2, 80, 139, 183, 36, 238, 9, 216, 200, 33, 144, 172, 9, 251, 45,
	160, 80, 242, 195, 231, 71, 130, 55, 224, 255, 242, 56, 194, 143, 19, 215, 151, 255, 254, 192,
	190, 132, 165, 3, 179, 254, 6, 176, 28, 241, 217, 0, 104, 28, 170, 105, 190, 55, 97, 102, 209,
	53, 247, 114, 34, 110, 191, 111, 215, 207, 180, 223, 87, 198, 125, 48, 150, 85, 255, 61, 214,
	247, 62, 133, 70, 245, 159, 45, 9, 239, 227, 201, 16, 215, 22, 126, 40, 231, 145, 174, 111,
	192, 72, 239, 200, 213, 239, 183, 173, 127, 241, 67, 166, 249, 202, 67, 136, 88, 163, 155, 11,
	181, 116, 129, 183, 197, 244, 226, 124, 134, 156, 102, 199, 94, 20, 43, 83, 40, 111, 29, 246,
	240, 2, 253, 117, 78, 64, 24, 69, 77, 46, 28, 24, 169, 8, 118, 32, 13, 246, 134, 45, 44, 125,
	87, 161, 10, 83, 226, 211, 165, 5, 77, 240, 4, 177, 254, 226, 148, 112, 107, 82, 126, 94, 86,
	212, 183, 169, 250, 83, 107,
];
pub const SET_THEA_KEY_PARAMETERS: ([u8; 64], [u8; 96], u128) = (
	[
		10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
		10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
		10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
	],
	[
		23, 66, 180, 26, 28, 111, 110, 214, 197, 11, 115, 191, 170, 132, 59, 193, 186, 37, 219, 80,
		65, 65, 179, 62, 17, 127, 79, 231, 14, 114, 179, 227, 143, 9, 247, 181, 188, 216, 107, 130,
		219, 233, 133, 203, 150, 7, 255, 153, 11, 159, 177, 244, 113, 3, 134, 137, 106, 109, 7, 64,
		86, 33, 1, 235, 14, 162, 238, 66, 216, 93, 192, 42, 192, 105, 161, 1, 82, 171, 206, 146,
		67, 193, 195, 159, 63, 114, 62, 48, 198, 226, 197, 249, 137, 32, 236, 18,
	],
	5,
);
pub const QUEUED_QUEUED_THEA_KEY_PARAMETERS: ([u8; 64], [u8; 96], u128) = (
	[
		10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
		10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
		10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
	],
	[
		23, 66, 180, 26, 28, 111, 110, 214, 197, 11, 115, 191, 170, 132, 59, 193, 186, 37, 219, 80,
		65, 65, 179, 62, 17, 127, 79, 231, 14, 114, 179, 227, 143, 9, 247, 181, 188, 216, 107, 130,
		219, 233, 133, 203, 150, 7, 255, 153, 11, 159, 177, 244, 113, 3, 134, 137, 106, 109, 7, 64,
		86, 33, 1, 235, 14, 162, 238, 66, 216, 93, 192, 42, 192, 105, 161, 1, 82, 171, 206, 146,
		67, 193, 195, 159, 63, 114, 62, 48, 198, 226, 197, 249, 137, 32, 236, 18,
	],
	5,
);
pub const CHANGE_THEA_KEY_PARAMETERS: ([u8; 96], u128) = (
	[
		9, 17, 126, 238, 113, 248, 97, 23, 1, 36, 99, 153, 116, 22, 182, 33, 40, 93, 154, 193, 70,
		239, 31, 100, 2, 50, 213, 203, 229, 157, 93, 130, 62, 254, 101, 217, 84, 20, 39, 160, 241,
		215, 215, 34, 197, 136, 75, 183, 20, 165, 222, 218, 209, 20, 231, 0, 132, 165, 146, 36, 39,
		162, 81, 14, 108, 14, 85, 54, 15, 195, 148, 22, 71, 72, 157, 63, 195, 174, 230, 50, 21,
		110, 144, 198, 111, 41, 144, 74, 46, 181, 36, 175, 50, 175, 98, 160,
	],
	6,
);

pub fn set_kth_bit(number: u128, k_value: u8) -> u128 {
	(1 << k_value) | number
}

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
			Origin::signed(1),
			Box::from(asset_id)
		));
		assert_ok!(Thea::approve_deposit(
			Origin::signed(1),
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
			Origin::signed(1),
			Box::from(asset_id)
		));
		assert_noop!(
			Thea::approve_deposit(
				Origin::signed(1),
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
			Origin::signed(1),
			Box::from(asset_id)
		));
		assert_noop!(
			Thea::approve_deposit(
				Origin::signed(1),
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
			Origin::signed(1),
			Box::from(asset_id)
		));
		assert_noop!(
			Thea::approve_deposit(
				Origin::signed(1),
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
			Origin::signed(1),
			Box::from(asset_id)
		));
		assert_noop!(
			Thea::approve_deposit(
				Origin::signed(1),
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
			Origin::signed(1),
			Box::from(asset_id)
		));
		assert_noop!(
			Thea::approve_deposit(
				Origin::signed(1),
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
				Origin::signed(1),
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
			Origin::signed(1),
			Box::from(asset_id.clone())
		));
		assert_ok!(Thea::set_withdrawal_fee(Origin::root(), 1, 0));
		let beneficiary: [u8; 32] = [1; 32];
		// Mint Asset to Alice
		assert_ok!(Balances::set_balance(Origin::root(), 1, 1_000_000_000_000, 0));
		assert_ok!(Assets::mint_into(generate_asset_id(asset_id.clone()), &1, 1_000_000_000_000));
		assert_ok!(Thea::withdraw(
			Origin::signed(1),
			generate_asset_id(asset_id.clone()),
			1000u128,
			beneficiary.to_vec(),
			false
		));
		let pending_withdrawal = <PendingWithdrawals<Test>>::get(1);
		let payload = ParachainWithdraw::get_parachain_withdraw(multi_asset, multi_location);
		let approved_withdraw = ApprovedWithdraw {
			asset_id: generate_asset_id(asset_id),
			amount: 1000,
			network: 1,
			beneficiary: vec![1; 32],
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
			Origin::signed(1),
			Box::from(asset_id.clone())
		));
		assert_ok!(Thea::set_withdrawal_fee(Origin::root(), 1, 0));
		let beneficiary: [u8; 32] = [1; 32];
		// Mint Asset to Alice
		assert_ok!(Balances::set_balance(Origin::root(), 1, 1_000_000_000_000, 0));
		assert_ok!(Assets::mint_into(generate_asset_id(asset_id.clone()), &1, 1_000_000_000_000));
		// bad origin test
		assert_err!(
			Thea::withdraw(
				Origin::none(),
				generate_asset_id(asset_id.clone()),
				1000u128,
				beneficiary.to_vec(),
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
				beneficiary.to_vec(),
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
				beneficiary.to_vec(),
				false
			),
			Error::<Test>::WithdrawalNotAllowed
		);
		<PendingWithdrawals<Test>>::insert(1, old_withdrawals);
		// good orogin test
		assert_ok!(Thea::withdraw(
			Origin::signed(1),
			generate_asset_id(asset_id.clone()),
			1000u128,
			beneficiary.to_vec(),
			false
		));
		let pending_withdrawal = <PendingWithdrawals<Test>>::get(1);
		let approved_withdraw = ApprovedWithdraw {
			asset_id: generate_asset_id(asset_id),
			amount: 1000,
			network: 1,
			beneficiary: vec![1; 32],
			payload: payload.encode(),
			index: 0,
		};
		assert_eq!(pending_withdrawal.to_vec().pop().unwrap(), approved_withdraw);
	})
}

#[test]
fn test_withdraw_with_wrong_benificiary_length() {
	new_test_ext().execute_with(|| {
		let beneficiary: [u8; 1000] = [1; 1000];
		assert_noop!(
			Thea::withdraw(Origin::signed(1), 1u128, 1000u128, beneficiary.to_vec(), false),
			Error::<Test>::BeneficiaryTooLong
		);
	})
}

#[test]
fn test_withdraw_with_wrong_asset_id_returns_unable_find_network_for_asset_id() {
	new_test_ext().execute_with(|| {
		let beneficiary: [u8; 32] = [1; 32];
		assert_noop!(
			Thea::withdraw(Origin::signed(1), 1u128, 1000u128, beneficiary.to_vec(), false),
			Error::<Test>::UnableFindNetworkForAssetId
		);
	})
}

#[test]
fn test_withdraw_with_no_fee_config() {
	new_test_ext().execute_with(|| {
		let beneficiary: [u8; 32] = [1; 32];
		let asset_id = AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here });
		assert_ok!(asset_handler::pallet::Pallet::<Test>::create_parachain_asset(
			Origin::signed(1),
			Box::from(asset_id.clone())
		));
		assert_noop!(
			Thea::withdraw(
				Origin::signed(1),
				generate_asset_id(asset_id),
				1000u128,
				beneficiary.to_vec(),
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
		let beneficiary: [u8; 32] = [1; 32];
		// Mint Asset to Alice
		assert_ok!(Balances::set_balance(Origin::root(), 1, 1_000_000_000_000_000_000, 0));
		assert_ok!(Thea::withdraw(
			Origin::signed(1),
			asset_id.clone(),
			10_000_000_000_000u128,
			beneficiary.to_vec(),
			false
		));
		let pending_withdrawal = <PendingWithdrawals<Test>>::get(1);
		let payload = ParachainWithdraw::get_parachain_withdraw(asset, multi_location);
		let approved_withdraw = ApprovedWithdraw {
			asset_id,
			amount: 10_000_000_000_000u128,
			network: 1,
			beneficiary: vec![1; 32],
			payload: payload.encode(),
			index: 0,
		};
		assert_eq!(pending_withdrawal.to_vec().pop().unwrap(), approved_withdraw);
	})
}

pub type PrivateKeys = Vec<SecretKey>;
pub type PublicKeys = Vec<BLSPublicKey>;

fn get_bls_keys() -> (PrivateKeys, PublicKeys) {
	let mut private_keys: PrivateKeys = vec![];
	let ikm = [0 as u8; 32];
	let sk_1 = SecretKey::key_gen(&ikm, &[]).unwrap();
	let pk_1 = sk_1.sk_to_pk();
	private_keys.push(sk_1.clone());
	let ikm = [1 as u8; 32];
	let sk_2 = SecretKey::key_gen(&ikm, &[]).unwrap();
	let pk_2 = sk_2.sk_to_pk();
	private_keys.push(sk_2.clone());
	let ikm = [2 as u8; 32];
	let sk_3 = SecretKey::key_gen(&ikm, &[]).unwrap();
	let pk_3 = sk_3.sk_to_pk();
	private_keys.push(sk_3.clone());
	let bls_public_key_1 = BLSPublicKey(pk_1.serialize().into());
	let bls_public_key_2 = BLSPublicKey(pk_2.serialize().into());
	let bls_public_key_3 = BLSPublicKey(pk_3.serialize().into());
	let public_keys: PublicKeys = vec![bls_public_key_1, bls_public_key_2, bls_public_key_3];
	(private_keys, public_keys)
}

fn register_bls_public_keys() {
	let (_, public_keys) = get_bls_keys();
	RelayersBLSKeyVector::<Test>::insert(1, public_keys);
}

fn sign_payload(payload: Vec<u8>) -> [u8; 96] {
	let (private_keys, _) = get_bls_keys();
	let sig_1 = private_keys[0].sign(&payload, DST, &[]);
	let sig_2 = private_keys[1].sign(&payload, DST, &[]);
	let mut agg_sig = AggregateSignature::from_signature(&sig_1);
	agg_sig.add_signature(&sig_2, false).unwrap();
	agg_sig.to_signature().serialize()
}

#[test]
fn test_withdrawal_returns_ok() {
	new_test_ext().execute_with(|| {
		let asset_id = AssetId::Concrete(MultiLocation { parents: 1, interior: Junctions::Here });
		assert_ok!(asset_handler::pallet::Pallet::<Test>::create_parachain_asset(
			Origin::signed(1),
			Box::from(asset_id.clone())
		));
		let asset_id = generate_asset_id(asset_id);
		assert_ok!(Balances::set_balance(Origin::root(), 1, 1_000_000_000_000, 0));
		assert_ok!(Assets::mint_into(asset_id, &1, 1000000000000u128));
		assert_ok!(Thea::set_withdrawal_fee(Origin::root(), 1, 0));
		assert_ok!(Thea::do_withdraw(1, asset_id, 1000000000u128, [1; 32].to_vec(), false));
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

fn create_account_id() -> AccountId32 {
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = KeyStore::new();
	let account_id: AccountId32 = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair")
	.try_into()
	.expect("Unable to convert to AccountId32");

	return account_id
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
	});
}

#[test]
fn batch_withdrawal_complete_works() {
	new_test_ext().execute_with(|| {
		// create
		let mut awd = vec![];
		let asset_id = H256::default();
		for i in 1..11 {
			awd.push(ApprovedWithdraw {
				asset_id: i,
				amount: i as u128,
				network: 1,
				beneficiary: vec![i as u8],
				payload: vec![i as u8],
				index: i as u32,
			});
		}
		let awd: BoundedVec<ApprovedWithdraw, ConstU32<10>> = awd.try_into().unwrap();
		<ReadyWithdrawls<Test>>::insert(1, 1, awd);
		// check
		assert!(!Thea::ready_withdrawals(1, 1).is_empty());
		// clean
		assert_ok!(Thea::batch_withdrawal_complete(
			Origin::signed(1),
			1,
			1,
			asset_id,
			1,
			[1 as u8; 96]
		));
		//check
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
		// relayer key insert
		let bls_key: BLSPublicKey = BLSPublicKey([1u8; 192]);
		<RelayersBLSKeyVector<Test>>::insert(1, vec![bls_key]);
		// authority insert
		<AuthorityListVector<Test>>::insert(1, vec![1]);
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
		// set up proper relayers and keys
		<RelayersBLSKeyVector<Test>>::insert(
			1,
			vec![
				BLSPublicKey(RELAYER_1_BLS_PUBLIC_KEY),
				BLSPublicKey(RELAYER_2_BLS_PUBLIC_KEY),
				BLSPublicKey(RELAYER_3_BLS_PUBLIC_KEY),
			],
		);
		<QueuedRelayersBLSKeyVector<Test>>::insert(
			1,
			vec![
				BLSPublicKey(RELAYER_1_BLS_PUBLIC_KEY),
				BLSPublicKey(RELAYER_2_BLS_PUBLIC_KEY),
				BLSPublicKey(RELAYER_3_BLS_PUBLIC_KEY),
			],
		);
		let (sig, map) = CHANGE_THEA_KEY_PARAMETERS;
		// test call succedes
		assert_ok!(Thea::thea_key_rotation_complete(Origin::signed(1), 1, H256::zero(), map, sig));
	});
}

#[test]
fn test_set_thea_key_complete() {
	new_test_ext().execute_with(|| {
		// relayer key insert
		let bls_key: BLSPublicKey = BLSPublicKey([1u8; 192]);
		<RelayersBLSKeyVector<Test>>::insert(1, vec![bls_key]);
		// authority insert
		<AuthorityListVector<Test>>::insert(1, vec![1]);
		// thea public key insert
		<TheaPublicKey<Test>>::insert(1, [1u8; 64]);
		// set up proper relayers and keys
		<RelayersBLSKeyVector<Test>>::insert(
			1,
			vec![
				BLSPublicKey(RELAYER_1_BLS_PUBLIC_KEY),
				BLSPublicKey(RELAYER_2_BLS_PUBLIC_KEY),
				BLSPublicKey(RELAYER_3_BLS_PUBLIC_KEY),
			],
		);
		<QueuedRelayersBLSKeyVector<Test>>::insert(
			1,
			vec![
				BLSPublicKey(RELAYER_1_BLS_PUBLIC_KEY),
				BLSPublicKey(RELAYER_2_BLS_PUBLIC_KEY),
				BLSPublicKey(RELAYER_3_BLS_PUBLIC_KEY),
			],
		);
		let (pk, sig, map) = SET_THEA_KEY_PARAMETERS;
		// test call no key fails as expected
		assert_err!(
			Thea::set_thea_key_complete(Origin::signed(1), 1, pk, map, sig),
			Error::<Test>::QueuedTheaPublicKeyNotFound
		);
		<TheaPublicKey<Test>>::insert(1, [1u8; 64]);
		// test call fails with bad signature
		assert_err!(
			Thea::set_thea_key_complete(Origin::signed(1), 1, [1u8; 64], 1, [1u8; 96]),
			Error::<Test>::BLSSignatureVerificationFailed
		);
		// test call success
		assert_ok!(Thea::set_thea_key_complete(Origin::signed(1), 1, pk, map, sig));
	});
}

#[test]
fn test_thea_queued_queued_public_key() {
	new_test_ext().execute_with(|| {
		let (pk, sig, map) = QUEUED_QUEUED_THEA_KEY_PARAMETERS;
		QueuedRelayers::<Test>::insert(
			1,
			vec![
				(1, RELAYER_1_BLS_PUBLIC_KEY),
				(2, RELAYER_2_BLS_PUBLIC_KEY),
				(3, RELAYER_3_BLS_PUBLIC_KEY),
			],
		);
		assert_ok!(Thea::thea_queued_queued_public_key(Origin::signed(1), 1, pk, map, sig));
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
