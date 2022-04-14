use frame_support::{assert_noop, assert_ok};
use sp_core::H256;
use sp_runtime::traits::{BadOrigin, BlockNumberProvider};

use crate::mock::{new_test_ext, Origin, PDEXMigration, Test, PDEX};

use crate::pallet::*;

#[test]
pub fn check_genesis_config() {
	new_test_ext().execute_with(|| {
		assert_eq!(PDEXMigration::operational(), false);
		assert_eq!(PDEXMigration::mintable_tokens(), 3_172_895 * PDEX);
	});
}

#[test]
pub fn set_migration_operational_status_works() {
	new_test_ext().execute_with(|| {
		let non_sudo = 2u64;
		assert_ok!(PDEXMigration::set_migration_operational_status(Origin::root(), true));
		assert_noop!(
			PDEXMigration::set_migration_operational_status(Origin::signed(non_sudo), false),
			BadOrigin,
		);
		assert_eq!(PDEXMigration::operational(), true);
		assert_ok!(PDEXMigration::set_migration_operational_status(Origin::root(), false));
		assert_eq!(PDEXMigration::operational(), false);
	});
}

#[test]
pub fn set_relayer_status_works() {
	new_test_ext().execute_with(|| {
		let relayer = 2u64;
		let non_relayer = 3u64;
		assert_ok!(PDEXMigration::set_relayer_status(Origin::root(), relayer, true));
		assert_eq!(Relayers::<Test>::get(&relayer), true);
		assert_ok!(PDEXMigration::set_relayer_status(Origin::root(), relayer, false));
		assert_eq!(Relayers::<Test>::get(&relayer), false);
		assert_eq!(Relayers::<Test>::get(&non_relayer), false);
	});
}

#[test]
pub fn unlock_tokens_works() {
	new_test_ext().execute_with(|| {
		let relayer1 = 21u64;
		let relayer2 = 22u64;
		let relayer3 = 23u64;
		let beneficiary = 4u64;
		let unknown_beneficiary = 5u64;
		let valid_amount = 100 * PDEX;
		let eth_hash = H256::random();
		assert_ok!(PDEXMigration::set_migration_operational_status(Origin::root(), true));
		// Register relayers
		assert_ok!(PDEXMigration::set_relayer_status(Origin::root(), relayer1, true));
		assert_ok!(PDEXMigration::set_relayer_status(Origin::root(), relayer2, true));
		assert_ok!(PDEXMigration::set_relayer_status(Origin::root(), relayer3, true));

		assert_ok!(PDEXMigration::mint(
			Origin::signed(relayer1),
			beneficiary,
			valid_amount,
			eth_hash
		));
		assert_ok!(PDEXMigration::mint(
			Origin::signed(relayer2),
			beneficiary,
			valid_amount,
			eth_hash
		));
		assert_ok!(PDEXMigration::mint(
			Origin::signed(relayer3),
			beneficiary,
			valid_amount,
			eth_hash
		));

		assert_noop!(
			PDEXMigration::unlock(Origin::signed(unknown_beneficiary)),
			Error::<Test>::UnknownBeneficiary
		);
		assert_noop!(
			PDEXMigration::unlock(Origin::signed(beneficiary)),
			Error::<Test>::LiquidityRestrictions
		);
	})
}

#[test]
pub fn remove_minted_tokens_works() {
	new_test_ext().execute_with(|| {
		let relayer1 = 21u64;
		let relayer2 = 22u64;
		let relayer3 = 23u64;
		let beneficiary = 4u64;
		let unknown_beneficiary = 5u64;
		let valid_amount = 100 * PDEX;
		let eth_hash = H256::random();
		assert_ok!(PDEXMigration::set_migration_operational_status(Origin::root(), true));
		// Register relayers
		assert_ok!(PDEXMigration::set_relayer_status(Origin::root(), relayer1, true));
		assert_ok!(PDEXMigration::set_relayer_status(Origin::root(), relayer2, true));
		assert_ok!(PDEXMigration::set_relayer_status(Origin::root(), relayer3, true));

		assert_ok!(PDEXMigration::mint(
			Origin::signed(relayer1),
			beneficiary,
			valid_amount,
			eth_hash
		));
		assert_ok!(PDEXMigration::mint(
			Origin::signed(relayer2),
			beneficiary,
			valid_amount,
			eth_hash
		));
		assert_ok!(PDEXMigration::mint(
			Origin::signed(relayer3),
			beneficiary,
			valid_amount,
			eth_hash
		));

		assert_noop!(
			PDEXMigration::unlock(Origin::signed(unknown_beneficiary)),
			Error::<Test>::UnknownBeneficiary
		);
		assert_noop!(
			PDEXMigration::unlock(Origin::signed(beneficiary)),
			Error::<Test>::LiquidityRestrictions
		);

		assert_eq!(MintableTokens::<Test>::get(), 3_172_895 * PDEX - valid_amount);
		assert_eq!(pallet_balances::Pallet::<Test>::total_issuance(), 100 * PDEX);

		// Remove the beneficiary's claim, minted tokens and increase the mintable tokens
		assert_ok!(PDEXMigration::remove_minted_tokens(Origin::root(), beneficiary));
		// Make sure beneficiary can't claim the tokens
		assert_noop!(
			PDEXMigration::unlock(Origin::signed(beneficiary)),
			Error::<Test>::UnknownBeneficiary
		);
		// Make sure the reduced mintable tokens is reverted
		assert_eq!(MintableTokens::<Test>::get(), 3_172_895 * PDEX);
		// Make sure the total supply is also decreased
		assert_eq!(pallet_balances::Pallet::<Test>::total_issuance(), 0 * PDEX);
	})
}

#[test]
pub fn mint_works() {
	new_test_ext().execute_with(|| {
		let relayer = 21u64;
		let relayer2 = 22u64;
		let relayer3 = 23u64;
		let non_relayer = 3u64;
		let beneficiary = 4u64;
		let invalid_amount = (3_172_895 + 1) * PDEX;
		let valid_amount = 100 * PDEX;
		let eth_hash = H256::random();
		assert_eq!(EthTxns::<Test>::get(eth_hash).approvals, 0);
		assert_eq!(EthTxns::<Test>::get(eth_hash).approvers.len(), 0);
		assert!(!EthTxns::<Test>::get(eth_hash).approvers.contains(&relayer));
		// Check if operational flag is working
		assert_noop!(
			PDEXMigration::mint(Origin::signed(relayer), beneficiary, valid_amount, eth_hash),
			Error::<Test>::NotOperational,
		);
		assert_ok!(PDEXMigration::set_migration_operational_status(Origin::root(), true));
		// Check if only registered relayers can call the mint function
		assert_noop!(
			PDEXMigration::mint(Origin::signed(non_relayer), beneficiary, valid_amount, eth_hash),
			Error::<Test>::UnknownRelayer,
		);
		assert_ok!(PDEXMigration::set_relayer_status(Origin::root(), relayer, true));
		// Ensure mint function cannot mint more than the amount available for migration
		assert_noop!(
			PDEXMigration::mint(Origin::signed(relayer), beneficiary, invalid_amount, eth_hash),
			Error::<Test>::InvalidMintAmount,
		);
		// Check if vote for a successful transaction is incremented
		let initial_total_issuance = pallet_balances::Pallet::<Test>::total_issuance();
		assert_ok!(PDEXMigration::mint(
			Origin::signed(relayer),
			beneficiary,
			valid_amount,
			eth_hash
		));
		assert_eq!(EthTxns::<Test>::get(&eth_hash).approvals, 1);
		assert_eq!(pallet_balances::Pallet::<Test>::total_issuance(), initial_total_issuance);
		// Ensure no new tokens are created yet
		// Register remaining two relayers
		assert_ok!(PDEXMigration::set_relayer_status(Origin::root(), relayer2, true));
		assert_ok!(PDEXMigration::mint(
			Origin::signed(relayer2),
			beneficiary,
			valid_amount,
			eth_hash
		));
		assert_eq!(EthTxns::<Test>::get(&eth_hash).approvals, 2);
		assert_eq!(pallet_balances::Pallet::<Test>::total_issuance(), initial_total_issuance);
		assert_ok!(PDEXMigration::set_relayer_status(Origin::root(), relayer3, true));
		assert_ok!(PDEXMigration::mint(
			Origin::signed(relayer3),
			beneficiary,
			valid_amount,
			eth_hash
		));
		assert_eq!(EthTxns::<Test>::get(&eth_hash).approvals, 3);
		// Ensure total issuance increased by valid_amount
		assert_eq!(
			pallet_balances::Pallet::<Test>::total_issuance(),
			initial_total_issuance + valid_amount
		);
		// Ensure the user cannot move the funds until unlocked
		assert_noop!(
			pallet_balances::Pallet::<Test>::transfer(
				Origin::signed(beneficiary),
				100,
				valid_amount - 1 * PDEX
			), // minus 1 PDEX is because of existential deposit
			pallet_balances::Error::<Test>::LiquidityRestrictions
		);
		// Unlock tokens should not work before lock period ends
		assert_eq!(PDEXMigration::unlock(Origin::signed(beneficiary)).is_err(), true);
		// progress block to 28 days lock
		frame_system::Pallet::<Test>::set_block_number(
			frame_system::Pallet::<Test>::current_block_number() + 201600,
		);
		// Unlock tokens
		assert_ok!(PDEXMigration::unlock(Origin::signed(beneficiary)));
		// check if it is transferable
		assert_ok!(pallet_balances::Pallet::<Test>::transfer(
			Origin::signed(beneficiary),
			100,
			valid_amount - 1 * PDEX
		));
		// Check balances
		assert_eq!(pallet_balances::Pallet::<Test>::free_balance(100), 99 * PDEX);
		assert_eq!(pallet_balances::Pallet::<Test>::free_balance(beneficiary), 1 * PDEX);
	});
}
