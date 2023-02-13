use crate::{
	mock::{new_test_ext, *},
	ActiveMembers, Candidates, Error,
};
use frame_support::{assert_noop, assert_ok};
use pallet_identity::{Data, IdentityInfo};
use sp_runtime::{traits::ConstU32, BoundedBTreeMap, BoundedVec};


type PublicKey = BoundedVec<u8, ConstU32<1000>>;
type KeysMap = BoundedBTreeMap<u8, PublicKey, ConstU32<20>>;

#[test]
fn test_apply_for_candidature_returns_ok() {
	new_test_ext().execute_with(|| {
		let candidate = 1;
		let key_map = get_keylist();
		assert_ok!(Balances::set_balance(
			Origin::root(),
			candidate,
			10_000_000_000_000_000u128,
			0
		));
		add_identity(candidate);
		assert_ok!(TheaGovernence::apply_for_candidature(Origin::signed(candidate), key_map));
	})
}

#[test]
fn test_apply_for_candidature_with_already_registered_member_returns_already_member_error() {
	new_test_ext().execute_with(|| {
		let key_map = get_keylist();
		let candidate = 1;
		<ActiveMembers<Test>>::insert(candidate, key_map.clone());
		add_identity(candidate);
		assert_noop!(
			TheaGovernence::apply_for_candidature(Origin::signed(candidate), key_map),
			Error::<Test>::AlreadyMember
		);
	})
}

#[test]
fn test_apply_for_candidature_with_already_registered_candidate_returns_already_applied_error() {
	new_test_ext().execute_with(|| {
		let key_map = get_keylist();
		let candidate = 1;
		<Candidates<Test>>::insert(candidate, key_map.clone());
		add_identity(candidate);
		assert_noop!(
			TheaGovernence::apply_for_candidature(Origin::signed(candidate), key_map),
			Error::<Test>::AlreadyApplied
		);
	})
}

#[test]
fn test_apply_for_candidature_with_not_enough_balnce_returns_error() {
	new_test_ext().execute_with(|| {
		let key_map = get_keylist();
		let candidate = 1;
		add_identity(candidate);
		assert_ok!(Balances::set_balance(Origin::root(), candidate, 0, 0));
		assert_noop!(
			TheaGovernence::apply_for_candidature(Origin::signed(candidate), key_map),
			pallet_balances::Error::<Test>::InsufficientBalance
		);
	})
}

#[test]
fn test_approve_candidature_returns_ok() {
	new_test_ext().execute_with(|| {
		let candidate = 1;
		let key_map = get_keylist();
		let general_council = 2;
		assert_ok!(Balances::set_balance(
			Origin::root(),
			candidate,
			10_000_000_000_000_000u128,
			0
		));
		add_identity(candidate);
		assert_ok!(TheaGovernence::apply_for_candidature(Origin::signed(candidate), key_map));
		assert_ok!(TheaGovernence::approve_candidature(
			Origin::signed(general_council),
			vec![candidate]
		));
	})
}

#[test]
fn test_approve_candidate_with_already_registered_member_returns_already_member_error() {
	new_test_ext().execute_with(|| {
		let candidate = 1;
		let general_council = 2;
		let key_map = get_keylist();
		<ActiveMembers<Test>>::insert(candidate, key_map);
		assert_noop!(
			TheaGovernence::approve_candidature(Origin::signed(general_council), vec![candidate]),
			Error::<Test>::AlreadyMember
		);
	})
}

#[test]
fn test_approve_candidate_with_wrong_candidate_returns_candidate_not_found_error() {
	new_test_ext().execute_with(|| {
		let un_registered_candidate = 1;
		let general_council = 2;
		assert_noop!(
			TheaGovernence::approve_candidature(
				Origin::signed(general_council),
				vec![un_registered_candidate]
			),
			Error::<Test>::CandidateNotFound
		);
	})
}

#[test]
fn test_add_new_keys_returns_ok() {
	new_test_ext().execute_with(|| {
		let candidate = 1;
		let key_map = get_keylist();
		let general_council = 2;
		assert_ok!(Balances::set_balance(
			Origin::root(),
			candidate,
			10_000_000_000_000_000u128,
			0
		));
		add_identity(candidate);
		assert_ok!(TheaGovernence::apply_for_candidature(Origin::signed(candidate), key_map));
		assert_ok!(TheaGovernence::approve_candidature(
			Origin::signed(general_council),
			vec![candidate]
		));
		let new_keys = get_keylist_arg(2);
		assert_ok!(TheaGovernence::add_new_keys(Origin::signed(candidate), new_keys));
	})
}

#[test]
fn test_add_new_keys_with_unregistered_member_returns_member_not_found() {
	new_test_ext().execute_with(|| {
		let unregistered_candidate = 1;
		let key_map = get_keylist_arg(2);
		assert_noop!(
			TheaGovernence::add_new_keys(Origin::signed(unregistered_candidate), key_map),
			Error::<Test>::MemberNotFound
		);
	})
}

#[test]
fn test_remove_candidate_returns_ok() {
	new_test_ext().execute_with(|| {
		let candidate = 1;
		let general_council = 2;
		assert_ok!(TheaGovernence::remove_candidate(
			Origin::signed(general_council),
			vec![candidate]
		));
	})
}

#[test]
fn test_approve_candidate_with_combo_of_right_and_wrong_candidates_returns_error() {
	new_test_ext().execute_with(|| {
		let candidate = 1;
		let key_map = get_keylist();
		let general_council = 2;
		let wrong_candidate = 10;
		assert_ok!(Balances::set_balance(
			Origin::root(),
			candidate,
			10_000_000_000_000_000u128,
			0
		));
		add_identity(candidate);
		assert_ok!(TheaGovernence::apply_for_candidature(Origin::signed(candidate), key_map));
		assert_noop!(
			TheaGovernence::approve_candidature(
				Origin::signed(general_council),
				vec![candidate, wrong_candidate]
			),
			Error::<Test>::CandidateNotFound
		);
		// Verify is first candidate is registered or not?
		assert!(!<ActiveMembers<Test>>::contains_key(candidate));
	})
}

#[test]
fn test_apply_for_candidature_without_identity_returns_identity_not_found() {
	new_test_ext().execute_with(|| {
		let candidate = 1;
		let key_map = get_keylist();
		let _general_council = 2;
		assert_ok!(Balances::set_balance(
			Origin::root(),
			candidate,
			10_000_000_000_000_000u128,
			0
		));
		assert_noop!(
			TheaGovernence::apply_for_candidature(Origin::signed(candidate), key_map),
			Error::<Test>::IdentityNotFound
		);
	})
}

fn add_identity(id: u64) {
	assert_ok!(Balances::set_balance(Origin::root(), id, 10_000_000_000_000_000u128, 0));
	assert_ok!(IdentityPallet::set_identity(Origin::signed(id), Box::new(ten())));
}

fn ten() -> IdentityInfo<MaxAdditionalFields> {
	IdentityInfo {
		additional: BoundedVec::default(),
		display: Data::Raw(b"ten".to_vec().try_into().unwrap()),
		legal: Data::Raw(b"The Right Ordinal Ten, Esq.".to_vec().try_into().unwrap()),
		web: Default::default(),
		riot: Default::default(),
		email: Default::default(),
		pgp_fingerprint: None,
		image: Default::default(),
		twitter: Default::default(),
	}
}

fn get_keylist() -> KeysMap {
	let identifier = 1;
	get_keylist_arg(identifier)
}

fn get_keylist_arg(idetifier: u8) -> KeysMap {
	let mut key_list: KeysMap = BoundedBTreeMap::<u8, PublicKey, ConstU32<20>>::new();
	let network_id = 1;
	let key: PublicKey = BoundedVec::try_from(vec![idetifier; 32]).unwrap();
	key_list.try_insert(network_id, key).unwrap();
	key_list
}
