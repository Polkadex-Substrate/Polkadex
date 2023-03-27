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
use frame_support::{assert_noop, assert_ok, error::BadOrigin, traits::fungibles::Mutate};
use parity_scale_codec::Encode;
use sp_core::{crypto::AccountId32, H160, H256};
use sp_keystore::{testing::KeyStore, SyncCryptoStore};
use sp_runtime::{traits::ConstU32, BoundedVec};
use thea_primitives::{
	parachain_primitives::{AssetType, ParachainAsset, ParachainDeposit, ParachainWithdraw},
	ApprovedWithdraw, BLSPublicKey, TokenType,
};
use xcm::{
	latest::{AssetId, Fungibility, Junction, Junctions, MultiAsset, MultiLocation, NetworkId},
	prelude::X1,
};

pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"ocex");
pub const DST: &[u8; 43] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";

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
fn test_withdraw_returns_ok() {
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
		assert_ok!(AssetHandler::mint_thea_asset(asset_id, 1, 1_000_000));
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
		// call extrinsic and check it passes
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

// hooks tests
#[test]
fn test_on_initialize() {
	new_test_ext().execute_with(|| {
		let msg = <IngressMessages<Test>>::get();
		assert!(msg.is_empty());
	});
}
