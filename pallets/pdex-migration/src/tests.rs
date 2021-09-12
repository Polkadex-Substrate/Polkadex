use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::BadOrigin;

use crate::mock::{new_test_ext, PDEX};
use crate::mock::{Origin, Test};
use crate::mock::PDEXMigration;

use super::*;

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
        assert_ok!(PDEXMigration::set_migration_operational_status(Origin::root(),true));
        assert_noop!(
			PDEXMigration::set_migration_operational_status(Origin::signed(non_sudo), false),
			BadOrigin,
		);
        assert_eq!(PDEXMigration::operational(), true);
        assert_ok!(PDEXMigration::set_migration_operational_status(Origin::root(),false));
        assert_eq!(PDEXMigration::operational(), false);
    });
}

#[test]
pub fn set_relayer_status_works() {
    new_test_ext().execute_with(|| {
        let relayer = 2u64;
        let non_relayer = 3u64;
        assert_ok!(PDEXMigration::set_relayer_status(Origin::root(),relayer,true));
        assert_eq!(Relayers::<Test>::get(&relayer), true);
        assert_ok!(PDEXMigration::set_relayer_status(Origin::root(),relayer,false));
        assert_eq!(Relayers::<Test>::get(&relayer), false);
        assert_eq!(Relayers::<Test>::get(&non_relayer), false);
    });
}

#[test]
pub fn mint_works() {
    new_test_ext().execute_with(|| {
        let relayer = 2u64;
        let non_relayer = 3u64;
        let beneficiary = 4u64;
        let invalid_amount = (3_172_895 + 1) * PDEX;
        let valid_amount = 1 * PDEX;

        assert_noop!(
			PDEXMigration::mint(Origin::signed(relayer), beneficiary,valid_amount),
			Error::<Test>::NotOperational,
		);
        assert_ok!(PDEXMigration::set_migration_operational_status(Origin::root(),true));
        assert_noop!(
			PDEXMigration::mint(Origin::signed(non_relayer), beneficiary,valid_amount),
			Error::<Test>::UnknownRelayer,
		);
        assert_ok!(PDEXMigration::set_relayer_status(Origin::root(),relayer,true));
        assert_noop!(
			PDEXMigration::mint(Origin::signed(relayer), beneficiary,invalid_amount),
			Error::<Test>::InvalidMintAmount,
		);
        assert_ok!(PDEXMigration::mint(Origin::signed(relayer), beneficiary,valid_amount));
        // Check lock
        // Check if it is transferable
        // progress block to 28 days lock
        // check if it is transferable
    });
}