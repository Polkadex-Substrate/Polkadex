use crate::{
	mock::*,
	session::{Exposure, IndividualExposure, StakingLimits, UnlockChunk},
	ActiveNetworks, Candidates, CurrentIndex, EraRewardPayout, Error, Hooks, Perbill, Stakers,
	Stakinglimits,
};
use frame_support::{assert_noop, assert_ok, traits::fungible::Mutate};
use polkadex_primitives::misbehavior::TheaMisbehavior;
use sp_runtime::traits::AccountIdConversion;
use std::collections::BTreeSet;
use thea_primitives::BLSPublicKey;

#[test]
fn test_add_candidate_with_valid_inputs_returns_ok() {
	new_test_ext().execute_with(|| {
		let candidate = 1;
		let network_id: u8 = 0;
		let bls_key: BLSPublicKey = BLSPublicKey([1u8; 192]);
		Balances::mint_into(&candidate, 10_000_000_000_000u128).unwrap();
		assert_ok!(TheaStaking::add_candidate(Origin::signed(candidate), network_id, bls_key));
		assert_eq!(Balances::free_balance(&candidate), 9_000_000_000_000);
		assert_eq!(Balances::reserved_balance(&candidate), 1_000_000_000_000);
		let exposure = Exposure {
			score: 1000,
			total: 1_000_000_000_000,
			individual: 1_000_000_000_000,
			bls_pub_key: bls_key,
			stakers: Default::default(),
		};
		assert_eq!(TheaStaking::candidates(network_id, candidate), Some(exposure));
		assert_eq!(TheaStaking::candidates_to_network(candidate), Some(network_id));
	});
}

#[test]
fn test_add_candidate_with_already_registered_candidate_returns_candidate_already_registered_error()
{
	new_test_ext().execute_with(|| {
		let candidate = 1;
		let network_id: u8 = 0;
		let bls_key: BLSPublicKey = BLSPublicKey([1u8; 192]);
		Balances::mint_into(&candidate, 10_000_000_000_000u128).unwrap();
		assert_ok!(TheaStaking::add_candidate(Origin::signed(candidate), network_id, bls_key));
		assert_noop!(
			TheaStaking::add_candidate(Origin::signed(candidate), network_id, bls_key),
			Error::<Test>::CandidateAlreadyRegistered
		);
	});
}

#[test]
fn test_add_candidate_with_low_free_balance_returns_low_balance_error() {
	new_test_ext().execute_with(|| {
		let candidate = 1;
		let network_id: u8 = 0;
		let bls_key: BLSPublicKey = BLSPublicKey([1u8; 192]);
		Balances::mint_into(&candidate, 10_000_000_000u128).unwrap();
		assert_noop!(
			TheaStaking::add_candidate(Origin::signed(candidate), network_id, bls_key),
			pallet_balances::Error::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn test_bound_with_valid_arguments_first_time_returns_ok() {
	new_test_ext().execute_with(|| {
		register_candidate();
		insert_staking_limit();
		// Give some Balance to Nominator
		let nominator = 2;
		Balances::mint_into(&nominator, 10_000_000_000_000u128).unwrap();
		assert_ok!(TheaStaking::bond(Origin::signed(nominator), 1_000_000_000_000u128, 1));
		let individual_exposure = IndividualExposure {
			who: nominator,
			value: 1_000_000_000_000u128,
			backing: 1,
			unlocking: vec![],
		};
		assert_eq!(TheaStaking::stakers(nominator), Some(individual_exposure));
	});
}

#[test]
fn test_bound_with_low_nominators_balance_returns_staking_limits_error() {
	new_test_ext().execute_with(|| {
		register_candidate();
		insert_staking_limit();
		let nominator = 2;
		assert_noop!(
			TheaStaking::bond(Origin::signed(nominator), 1_000_000_000_00u128, 1),
			Error::<Test>::StakingLimitsError
		);
	});
}

#[test]
fn test_bound_with_low_nominators_balance_return_insufficient_balance() {
	new_test_ext().execute_with(|| {
		register_candidate();
		insert_staking_limit();
		let nominator = 2;
		assert_noop!(
			TheaStaking::bond(Origin::signed(nominator), 1_000_000_000_000u128, 1),
			pallet_balances::Error::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn test_nominate_with_valid_arguments_returns_ok() {
	new_test_ext().execute_with(|| {
		register_candidate();
		insert_staking_limit();
		register_nominator();
		let (candidate, network_id, bls_key) = get_candidate();
		let nominator = 2;
		let mut stakers: BTreeSet<u64> = BTreeSet::new();
		stakers.insert(nominator);
		let exposure = Exposure {
			score: 1000,
			total: 2_000_000_000_000,
			individual: 1_000_000_000_000,
			bls_pub_key: bls_key,
			stakers,
		};
		assert_eq!(TheaStaking::candidates(network_id, candidate), Some(exposure));
		let nominator_exposure = IndividualExposure {
			who: nominator,
			value: 1_000_000_000_000u128,
			backing: candidate,
			unlocking: vec![],
		};
		assert_eq!(TheaStaking::stakers(nominator), Some(nominator_exposure));
	});
}
#[test]
fn test_nominate_with_invalid_nominator_returns_staker_not_found() {
	new_test_ext().execute_with(|| {
		let nominator = 2;
		let candidate = 1;
		assert_noop!(
			TheaStaking::nominate(Origin::signed(nominator), candidate),
			Error::<Test>::StakerNotFound
		);
	})
}

#[test]
fn test_nominate_with_already_staked_relayer_returns_staker_already_nominating() {
	new_test_ext().execute_with(|| {
		register_candidate();
		insert_staking_limit();
		register_nominator();
		let (candidate, ..) = get_candidate();
		let nominator = 2;
		assert_noop!(
			TheaStaking::nominate(Origin::signed(nominator), candidate),
			Error::<Test>::CandidateAlreadyNominated
		);
		assert_noop!(
			TheaStaking::nominate(Origin::signed(nominator), 2),
			Error::<Test>::StakerAlreadyNominating
		);
	})
}

// as now binding happens instantly without option this test now tests
// that re-nominating is not happening on second nomination
#[test]
fn test_nominate_with_wrong_candidate_returns_candidate_already_nominated() {
	new_test_ext().execute_with(|| {
		insert_staking_limit();
		register_candidate();
		register_nominator();
		let (candidate, ..) = get_candidate();
		let nominator = 2;
		assert_noop!(
			TheaStaking::nominate(Origin::signed(nominator), candidate),
			Error::<Test>::CandidateAlreadyNominated
		);
	});
}

#[test]
fn test_bound_with_valid_arguments_second_time_returns_ok() {
	new_test_ext().execute_with(|| {
		register_candidate();
		insert_staking_limit();
		// Give some Balance to Nominator
		let nominator = 2;
		Balances::mint_into(&nominator, 10_000_000_000_000u128).unwrap();
		assert_ok!(TheaStaking::bond(Origin::signed(nominator), 1_000_000_000_000u128, 1));
		assert_ok!(TheaStaking::bond(Origin::signed(nominator), 1_000_000_000_000u128, 1));
		let individual_exposure = IndividualExposure {
			who: nominator,
			value: 2_000_000_000_000u128,
			backing: 1,
			unlocking: vec![],
		};
		assert_eq!(TheaStaking::stakers(nominator), Some(individual_exposure));
	});
}

#[test]
fn test_unbond_with_valid_arguments_returns_ok() {
	new_test_ext().execute_with(|| {
		register_candidate();
		insert_staking_limit();
		register_nominator();
		let (candidate, network, bls_key) = get_candidate();
		let nominator = 2;
		assert_ok!(TheaStaking::unbond(Origin::signed(nominator), 1_00_000_000_000));
		let mut stakers: BTreeSet<u64> = BTreeSet::new();
		stakers.insert(nominator);
		let relayer_exposure = Exposure {
			score: 1000,
			total: 1_900_000_000_000u128,
			individual: 1_000_000_000_000_u128,
			bls_pub_key: bls_key,
			stakers,
		};
		assert_eq!(TheaStaking::candidates(network, candidate), Some(relayer_exposure));
	})
}

#[test]
fn test_unbond_with_unregistered_nominator_returns_staker_not_found_error() {
	new_test_ext().execute_with(|| {
		register_candidate();
		insert_staking_limit();
		let nominator = 2u64;
		assert_noop!(
			TheaStaking::unbond(Origin::signed(nominator), 1_000_000_000_000),
			Error::<Test>::StakerNotFound
		);
	})
}

#[test]
fn test_unbond_with_zero_nomination_returns_ok() {
	new_test_ext().execute_with(|| {
		register_candidate();
		insert_staking_limit();
		register_nominator();
		let nominator = 2u64;
		assert_ok!(TheaStaking::unbond(Origin::signed(nominator), 1_00_000_000_000));
		let unlocking_chunk = UnlockChunk { value: 1_00_000_000_000, era: 10 };
		let nominator_exposure = IndividualExposure {
			who: nominator,
			value: 1_000_000_000_000,
			backing: 1,
			unlocking: vec![unlocking_chunk],
		};
		assert_eq!(TheaStaking::stakers(nominator), Some(nominator_exposure));
	})
}

#[test]
fn test_withdraw_unbounded_with_returns_ok() {
	new_test_ext().execute_with(|| {
		register_candidate();
		insert_staking_limit();
		register_nominator();
		unbonding();
		let nominator = 2u64;
		let current_session = CurrentIndex::<Test>::get();
		let current_session = current_session.saturating_add(10);

		CurrentIndex::<Test>::set(current_session);
		assert_ok!(TheaStaking::withdraw_unbonded(Origin::signed(nominator)));
	})
}

#[test]
fn test_withdraw_unbouded_with_unregistered_nominator_returns_error() {
	new_test_ext().execute_with(|| {
		let nominator = 2u64;
		assert_noop!(
			TheaStaking::withdraw_unbonded(Origin::signed(nominator)),
			Error::<Test>::CandidateNotFound
		);
	})
}

#[test]
fn test_remove_candidate_with_right_arguments_returns_ok() {
	new_test_ext().execute_with(|| {
		register_candidate();
		let (candidate, network, ..) = get_candidate();
		assert_ok!(TheaStaking::remove_candidate(Origin::signed(candidate), network));
	})
}

#[test]
fn test_remove_candidate_with_wrong_netowork_id_returns_error() {
	new_test_ext().execute_with(|| {
		register_candidate();
		let (candidate, ..) = get_candidate();
		let wrong_network_id = 5;
		assert_noop!(
			TheaStaking::remove_candidate(Origin::signed(candidate), wrong_network_id),
			Error::<Test>::CandidateNotFound
		);
	})
}

#[test]
fn test_remove_candidate_with_unregistered_nominator_returns_error() {
	new_test_ext().execute_with(|| {
		let (candidate, network_id, ..) = get_candidate();
		assert_noop!(
			TheaStaking::remove_candidate(Origin::signed(candidate), network_id),
			Error::<Test>::CandidateNotFound
		);
	})
}

#[test]
fn test_elect_relayers_with_candidates_less_than_max_candidates_allowed_returns_all_provided_candidates(
) {
	new_test_ext().execute_with(|| {
		insert_staking_limit();
		let candidate_one = 1u64;
		let exposure_for_candidate_one = Exposure {
			score: 1000,
			total: 1_000_000_000_000u128,
			individual: 1_000_000_000_000u128,
			bls_pub_key: BLSPublicKey([1u8; 192]),
			stakers: Default::default(),
		};
		let candidate_two = 2u64;
		let exposure_for_candidate_two = Exposure {
			score: 1000,
			total: 1_000_000_000_000u128,
			individual: 1_000_000_000_000u128,
			bls_pub_key: BLSPublicKey([1u8; 192]),
			stakers: Default::default(),
		};
		let actual_candidates_list = vec![
			(candidate_one, exposure_for_candidate_one.clone()),
			(candidate_two, exposure_for_candidate_two.clone()),
		];
		let expected_candidate_list = vec![
			(candidate_one, exposure_for_candidate_one),
			(candidate_two, exposure_for_candidate_two),
		];
		assert_eq!(crate::elect_relayers::<Test>(actual_candidates_list), expected_candidate_list);
	})
}

#[test]
fn test_elect_relayers_with_candidates_more_than_max_candidates_allowed_returns_candidates_with_more_stake(
) {
	new_test_ext().execute_with(|| {
		insert_staking_limit();
		let candidate_one = 1u64;
		let exposure_for_candidate_one = Exposure {
			score: 1000,
			total: 1_000_000_000_000u128,
			individual: 1_000_000_000_000u128,
			bls_pub_key: BLSPublicKey([1u8; 192]),
			stakers: Default::default(),
		};
		let candidate_two = 2u64;
		let exposure_for_candidate_two = Exposure {
			score: 1000,
			total: 2_000_000_000_000u128,
			individual: 2_000_000_000_000u128,
			bls_pub_key: BLSPublicKey([1u8; 192]),
			stakers: Default::default(),
		};
		let candidate_three = 3u64;
		let exposure_for_candidate_three = Exposure {
			score: 1000,
			total: 3_000_000_000_000u128,
			individual: 3_000_000_000_000u128,
			bls_pub_key: BLSPublicKey([1u8; 192]),
			stakers: Default::default(),
		};
		let actual_candidate_list = vec![
			(candidate_one, exposure_for_candidate_one),
			(candidate_two, exposure_for_candidate_two.clone()),
			(candidate_three, exposure_for_candidate_three.clone()),
		];
		let expected_candidate_list = vec![
			(candidate_three, exposure_for_candidate_three),
			(candidate_two, exposure_for_candidate_two),
		];
		assert_eq!(crate::elect_relayers::<Test>(actual_candidate_list), expected_candidate_list);
	})
}
//TODO: Should we also check if BLS Key is already registered or not?

#[test]
fn test_compute_next_session_with_valid_arguments() {
	new_test_ext().execute_with(|| {
		insert_staking_limit();
		register_candidate();
		register_new_candidate(2, 0, BLSPublicKey([2u8; 192]));
		let candidate_one_exposure = Exposure {
			score: 1000,
			total: 1_000_000_000_000u128,
			individual: 1_000_000_000_000u128,
			bls_pub_key: BLSPublicKey([1u8; 192]),
			stakers: Default::default(),
		};
		let candidate_two_exposure = Exposure {
			score: 1000,
			total: 1_000_000_000_000u128,
			individual: 1_000_000_000_000u128,
			bls_pub_key: BLSPublicKey([2u8; 192]),
			stakers: Default::default(),
		};
		let current_session = 0;
		let session_in_consideration = 2;
		let current_network = 0;
		TheaStaking::compute_next_session(current_network, current_session);
		let actual_staking_data =
			TheaStaking::staking_data(session_in_consideration, current_network);
		let expected_staking_data =
			vec![(1u64, candidate_one_exposure), (2u64, candidate_two_exposure)];
		assert_eq!(actual_staking_data, expected_staking_data);
		let actual_queued_candidates = TheaStaking::queued_relayers(current_network);
		let expected_queued_candidates =
			vec![(1, BLSPublicKey([1u8; 192])), (2, BLSPublicKey([2u8; 192]))];
		assert_eq!(actual_queued_candidates, expected_queued_candidates);
	})
}

#[test]
fn test_rotate_session() {
	new_test_ext().execute_with(|| {
		rotate_session_init();
		let candidate_one = 1u64;
		let candidate_two = 2u64;
		assert_eq!(TheaStaking::active_relayers(0), vec![]);
		TheaStaking::rotate_session();
		assert_eq!(
			TheaStaking::queued_relayers(0),
			vec![
				(candidate_one, BLSPublicKey([1u8; 192])),
				(candidate_two, BLSPublicKey([2u8; 192]))
			]
		);
		assert_eq!(TheaStaking::active_relayers(0), vec![]);
		TheaStaking::rotate_session(); //Update current session
		assert_eq!(
			TheaStaking::active_relayers(0),
			vec![
				(candidate_one, BLSPublicKey([1u8; 192])),
				(candidate_two, BLSPublicKey([2u8; 192]))
			]
		);
	})
}

fn rotate_session_init() {
	let current_session = 1;
	<CurrentIndex<Test>>::put(current_session);
	let mut set = BTreeSet::new();
	set.insert(0);
	<ActiveNetworks<Test>>::put(set);
	register_candidate();
	register_new_candidate(2, 0, BLSPublicKey([2; 192]));
	insert_staking_limit();
}

#[test]
fn test_unbond_with_amount_more_than_staked_amount_returns_error() {
	new_test_ext().execute_with(|| {
		register_candidate();
		insert_staking_limit();
		register_nominator();
		let nominator = 2u64;
		assert_noop!(
			TheaStaking::unbond(Origin::signed(nominator), 1000_000_000_000_000),
			Error::<Test>::AmountIsGreaterThanBondedAmount
		);
	})
}

#[test]
fn test_unbond_with_amount_equal_to_staked_amount_returns_ok() {
	new_test_ext().execute_with(|| {
		register_candidate();
		insert_staking_limit();
		register_nominator();
		let nominator = 2u64;
		let candidate = 1u64;
		let network_id = 0;
		let bls_key = BLSPublicKey([1; 192]);
		assert_ok!(TheaStaking::unbond(Origin::signed(nominator), 1_000_000_000_000u128));
		let stakers: BTreeSet<u64> = BTreeSet::new();
		let exposure = Exposure {
			score: 1000,
			total: 1_000_000_000_000,
			individual: 1_000_000_000_000,
			bls_pub_key: bls_key,
			stakers,
		};
		assert_eq!(TheaStaking::candidates(network_id, candidate), Some(exposure));
		let nominator_exposure = IndividualExposure {
			who: nominator,
			value: 1_000_000_000_000u128,
			backing: 1,
			unlocking: vec![UnlockChunk { value: 1000000000000, era: 10 }],
		};
		assert_eq!(TheaStaking::stakers(nominator), Some(nominator_exposure));
	})
}

use thea_primitives::TheaExtrinsicSubmitted;
const SESSION_LENGTH: u32 = 7000;
#[test]
fn test_reward_payout() {
	new_test_ext().execute_with(|| {
		register_candidate();
		insert_staking_limit();
		let initial_balance = Balances::free_balance(1);
		let mut active_set = BTreeSet::new();
		active_set.insert(0);
		ActiveNetworks::<Test>::put(active_set);
		assert_eq!(CurrentIndex::<Test>::get(), 0);
		TheaStaking::on_initialize(SESSION_LENGTH.into());
		assert_eq!(CurrentIndex::<Test>::get(), 1);
		TheaStaking::on_initialize(SESSION_LENGTH.into());
		TheaStaking::on_initialize(SESSION_LENGTH.into());
		TheaStaking::thea_extrinsic_submitted(1, 0, vec![]);
		TheaStaking::on_initialize(SESSION_LENGTH.into());
		let reward = EraRewardPayout::<Test>::get(3);
		assert_ok!(TheaStaking::stakers_payout(Origin::signed(1), 3));
		let reward_received = Balances::free_balance(1) - initial_balance;
		assert!(reward_received <= reward);
	})
}

const PDEX: u128 = 1_000_000_000_000;

#[test]
fn test_reward_with_nominators() {
	new_test_ext().execute_with(|| {
		let mut active_networks = BTreeSet::new();
		active_networks.insert(1_u8);
		ActiveNetworks::<Test>::set(active_networks);
		Balances::mint_into(&11, 2 * PDEX).unwrap();
		assert_ok!(Balances::mint_into(&10, 1000000 * PDEX));
		assert_ok!(TheaStaking::add_candidate(Origin::signed(11), 1, BLSPublicKey([0_u8; 192])));
		let _alice_balances = Balances::free_balance(11);
		Balances::mint_into(&21, 3 * PDEX).unwrap();
		assert_ok!(TheaStaking::add_candidate(Origin::signed(21), 1, BLSPublicKey([0_u8; 192])));
		let _bob_balances = Balances::free_balance(21);
		Balances::mint_into(&101, 10000 * PDEX).unwrap();
		assert_ok!(TheaStaking::bond(Origin::signed(101), 10000 * PDEX, 11));
		let _nominator_balances = Balances::free_balance(101);
		let _nominator_exposure = Stakers::<Test>::get(101).unwrap();
		let alice_exposure = Candidates::<Test>::get(1, 11).unwrap();
		let _alice_part = Perbill::from_rational(alice_exposure.individual, alice_exposure.total);
		// FIXME: Current implementation does not support one nominator
		// nominating multiple relayers
		TheaStaking::on_initialize(SESSION_LENGTH.into());
		TheaStaking::on_initialize(SESSION_LENGTH.into());
		TheaStaking::thea_extrinsic_submitted(11, 0, vec![]);
		TheaStaking::thea_extrinsic_submitted(11, 0, vec![]);
		TheaStaking::thea_extrinsic_submitted(21, 0, vec![]);
		TheaStaking::on_initialize(SESSION_LENGTH.into());
		assert_eq!(CurrentIndex::<Test>::get(), 3);

		let initial_alice_balance = Balances::free_balance(11);
		let initial_bob_balance = Balances::free_balance(21);
		let initial_nominator_balance = Balances::free_balance(101);

		let reward = EraRewardPayout::<Test>::get(2);

		assert_ok!(TheaStaking::stakers_payout(Origin::signed(11), 2));
		assert_ok!(TheaStaking::stakers_payout(Origin::signed(21), 2));

		let alice_balances = Balances::free_balance(11);
		let nominator_balances = Balances::free_balance(101);
		let bob_balances = Balances::free_balance(21);

		let alice_reward = alice_balances - initial_alice_balance;
		let bob_reward = bob_balances - initial_bob_balance;
		let nominator_reward = nominator_balances - initial_nominator_balance;

		let reward_paid = alice_reward + bob_reward + nominator_reward;

		assert!(reward_paid <= reward);
	})
}

// Start balance of all candidates for misbehavior testing
const START_BALANCE: u128 = 100 * PDEX;
// Offence
const OFFENCE: TheaMisbehavior = TheaMisbehavior::FaultyDataProvided;
// Severe Offence
const SEVERE_OFFENCE: TheaMisbehavior = TheaMisbehavior::UnattendedKeygen;

const ALICE_ACCOUNT: u64 = 1;
const BOB_ACCOUNT: u64 = 2;
const NEAL_ACCOUNT: u64 = 3;
const NEAL_ACCOUNT_NOMINATOR_1: u64 = 10;
const NEAL_ACCOUNT_NOMINATOR_2: u64 = 11;

fn misbehavior_setup_as_alice_bob_and_neal_in_relayer_set() {
	let mut active_networks = BTreeSet::new();
	active_networks.insert(1_u8);
	ActiveNetworks::<Test>::set(active_networks);
	// Alice start balance
	Balances::mint_into(&ALICE_ACCOUNT, START_BALANCE).unwrap();
	// Bob start balance
	Balances::mint_into(&BOB_ACCOUNT, START_BALANCE).unwrap();
	// Charles start balance
	Balances::mint_into(&NEAL_ACCOUNT, START_BALANCE).unwrap();
	// Alice candidate
	TheaStaking::add_candidate(Origin::signed(ALICE_ACCOUNT), 1, BLSPublicKey([0_u8; 192]))
		.unwrap();
	// Bob candidate
	TheaStaking::add_candidate(Origin::signed(BOB_ACCOUNT), 1, BLSPublicKey([2_u8; 192])).unwrap();
	// Neal candidate
	TheaStaking::add_candidate(Origin::signed(NEAL_ACCOUNT), 1, BLSPublicKey([3_u8; 192])).unwrap();
	// Neal's Nominator 1
	Balances::mint_into(&NEAL_ACCOUNT_NOMINATOR_1, 10000 * NEAL_ACCOUNT_NOMINATOR_1 as u128 * PDEX)
		.unwrap();
	assert_ok!(TheaStaking::bond(
		Origin::signed(NEAL_ACCOUNT_NOMINATOR_1),
		10000 * NEAL_ACCOUNT_NOMINATOR_1 as u128 * PDEX,
		NEAL_ACCOUNT
	));
	// Neal's Nominator 2
	Balances::mint_into(&NEAL_ACCOUNT_NOMINATOR_2, 10000 * NEAL_ACCOUNT_NOMINATOR_2 as u128 * PDEX)
		.unwrap();
	assert_ok!(TheaStaking::bond(
		Origin::signed(NEAL_ACCOUNT_NOMINATOR_2),
		10000 * NEAL_ACCOUNT_NOMINATOR_2 as u128 * PDEX,
		NEAL_ACCOUNT
	));
}

fn misbehavior_setup_three_candidates_two_nominators() {
	let mut active_networks = BTreeSet::new();
	active_networks.insert(1_u8);
	ActiveNetworks::<Test>::set(active_networks);
	// A start balance
	Balances::mint_into(&1, START_BALANCE).unwrap();
	// B start balance
	Balances::mint_into(&2, START_BALANCE).unwrap();
	// C start balance
	Balances::mint_into(&3, START_BALANCE).unwrap();
	// A candidate
	TheaStaking::add_candidate(Origin::signed(1), 1, BLSPublicKey([0_u8; 192])).unwrap();
	// B candidate
	TheaStaking::add_candidate(Origin::signed(2), 1, BLSPublicKey([2_u8; 192])).unwrap();
	// C candidate
	TheaStaking::add_candidate(Origin::signed(3), 1, BLSPublicKey([3_u8; 192])).unwrap();
	for id in 10..=11 {
		Balances::mint_into(&id, 10000 * id as u128 * PDEX).unwrap();
		assert_ok!(TheaStaking::bond(Origin::signed(id), 10000 * id as u128 * PDEX, 3));
	}
}

#[test]
fn test_reporting_misbehavior_works() {
	new_test_ext().execute_with(|| {
		misbehavior_setup_three_candidates_two_nominators();
		// We fail as those are not in active set yet
		assert!(TheaStaking::report_offence(Origin::signed(1), 1, 3, OFFENCE).is_err());

		//TheaStaking::on_initialize(SESSION_LENGTH.into());
		TheaStaking::rotate_session();
		TheaStaking::rotate_session();
		// Now shold be ok
		TheaStaking::report_offence(Origin::signed(1), 1, 3, OFFENCE).unwrap();
	});
}

#[test]
fn test_slashing_misbehavior_works() {
	new_test_ext().execute_with(|| {
		misbehavior_setup_as_alice_bob_and_neal_in_relayer_set();

		let treasury_pallet_account = TreasuryPalletId::get().into_account_truncating();
		//pallet needs to have an existential deposit
		Balances::mint_into(&treasury_pallet_account, PDEX).unwrap();
		let treasury = Balances::free_balance(&treasury_pallet_account);
		assert_eq!(treasury, PDEX);

		// We fail as those are not in active set yet
		assert!(TheaStaking::report_offence(
			Origin::signed(ALICE_ACCOUNT),
			1,
			NEAL_ACCOUNT,
			OFFENCE
		)
		.is_err());

		// make sure treasury is empty
		TheaStaking::rotate_session();
		TheaStaking::rotate_session();

		// Report Neal as offensive relayer
		TheaStaking::report_offence(Origin::signed(ALICE_ACCOUNT), 1, NEAL_ACCOUNT, OFFENCE)
			.unwrap();
		TheaStaking::report_offence(Origin::signed(BOB_ACCOUNT), 1, NEAL_ACCOUNT, OFFENCE).unwrap();

		//get alice and bob free balance
		let alice_free_balance = Balances::free_balance(ALICE_ACCOUNT);
		let bob_free_balance = Balances::free_balance(BOB_ACCOUNT);
		let neal_free_balance = Balances::free_balance(BOB_ACCOUNT);

		assert_eq!(alice_free_balance, 99000000000000);
		assert_eq!(bob_free_balance, 99000000000000);
		assert_eq!(neal_free_balance, 99000000000000);

		let neal_nominator_1_reserve_balance = Balances::reserved_balance(NEAL_ACCOUNT_NOMINATOR_1);
		let neal_nominator_2_reserve_balance = Balances::reserved_balance(NEAL_ACCOUNT_NOMINATOR_2);
		let neal_reserve_balance_before_slashing = Balances::reserved_balance(NEAL_ACCOUNT);
		assert_eq!(neal_nominator_1_reserve_balance, 100000000000000000);
		assert_eq!(neal_nominator_2_reserve_balance, 110000000000000000);
		assert_eq!(neal_reserve_balance_before_slashing, 1000000000000);

		let treasury_balance_before_slashing = Balances::free_balance(&treasury_pallet_account);
		assert_eq!(treasury_balance_before_slashing, PDEX);

		// Rotate for slashing to take place
		TheaStaking::rotate_session();
		// Make sure storage is cleaned up
		assert!(TheaStaking::commited_slashing(3).1.is_empty());

		// get alice and bob free balance after slashing
		let alice_free_balance_after_rewarding = Balances::free_balance(ALICE_ACCOUNT);
		let bob_free_balance_after_rewarding = Balances::free_balance(BOB_ACCOUNT);
		// Alice and bob free balance increases as as they reported neal's offensive behaviour
		assert_eq!(alice_free_balance_after_rewarding, 151500250000000);
		assert_eq!(bob_free_balance_after_rewarding, 151500250000000);

		// get neal's nominators as well as neal's after slashing
		let neal_nominator_1_reserve_balance_after_slashing =
			Balances::reserved_balance(NEAL_ACCOUNT_NOMINATOR_1);
		let neal_nominator_2_reserve_balance_after_slashing =
			Balances::reserved_balance(NEAL_ACCOUNT_NOMINATOR_2);
		let neal_reserve_balance_after_slashing = Balances::reserved_balance(NEAL_ACCOUNT);
		// As neal was reported for offensive behaviour, neal as well as neal nominators got slashed
		assert_eq!(neal_nominator_1_reserve_balance_after_slashing, 95000000000000000);
		assert_eq!(neal_nominator_2_reserve_balance_after_slashing, 104500000000000000);
		assert_eq!(neal_reserve_balance_after_slashing, 950000000000);

		let slashed_amount_nom_1 = neal_nominator_1_reserve_balance
			.saturating_sub(neal_nominator_1_reserve_balance_after_slashing);
		let slashed_amount_nom_2 = neal_nominator_2_reserve_balance
			.saturating_sub(neal_nominator_2_reserve_balance_after_slashing);
		let slashed_amount_neal = neal_reserve_balance_before_slashing
			.saturating_sub(neal_reserve_balance_after_slashing);

		let treasury_balance_after_getting_rewarded_from_slash =
			Balances::free_balance(&treasury_pallet_account);
		assert_eq!(treasury_balance_after_getting_rewarded_from_slash, 10396049500000000);

		let total_slashed = slashed_amount_nom_1
			.saturating_add(slashed_amount_nom_2.saturating_add(slashed_amount_neal));
		let alice_gain = alice_free_balance_after_rewarding.saturating_sub(alice_free_balance);
		let bob_gain = bob_free_balance_after_rewarding.saturating_sub(bob_free_balance);
		let pallet_gain = treasury_balance_after_getting_rewarded_from_slash
			.saturating_sub(treasury_balance_before_slashing);
		let total_gain = alice_gain.saturating_add(bob_gain.saturating_add(pallet_gain));

		assert_eq!(total_slashed, total_gain);
	});
}

#[test]
fn test_slashing_severe_misbehavior_works() {
	new_test_ext().execute_with(|| {
		misbehavior_setup_as_alice_bob_and_neal_in_relayer_set();

		let treasury_pallet_account = TreasuryPalletId::get().into_account_truncating();
		//pallet needs to have an existential deposit
		Balances::mint_into(&treasury_pallet_account, PDEX).unwrap();
		let treasury = Balances::free_balance(&treasury_pallet_account);
		assert_eq!(treasury, PDEX);

		// We fail as those are not in active set yet
		assert!(TheaStaking::report_offence(
			Origin::signed(ALICE_ACCOUNT),
			1,
			NEAL_ACCOUNT,
			SEVERE_OFFENCE
		)
		.is_err());

		// make sure treasury is empty
		TheaStaking::rotate_session();
		TheaStaking::rotate_session();

		// Report Neal as offensive relayer
		TheaStaking::report_offence(Origin::signed(ALICE_ACCOUNT), 1, NEAL_ACCOUNT, SEVERE_OFFENCE)
			.unwrap();
		TheaStaking::report_offence(Origin::signed(BOB_ACCOUNT), 1, NEAL_ACCOUNT, SEVERE_OFFENCE)
			.unwrap();

		//get alice and bob free balance
		let alice_free_balance = Balances::free_balance(ALICE_ACCOUNT);
		let bob_free_balance = Balances::free_balance(BOB_ACCOUNT);
		let neal_free_balance = Balances::free_balance(BOB_ACCOUNT);

		assert_eq!(alice_free_balance, 99000000000000);
		assert_eq!(bob_free_balance, 99000000000000);
		assert_eq!(neal_free_balance, 99000000000000);

		let neal_nominator_1_reserve_balance = Balances::reserved_balance(NEAL_ACCOUNT_NOMINATOR_1);
		let neal_nominator_2_reserve_balance = Balances::reserved_balance(NEAL_ACCOUNT_NOMINATOR_2);
		let neal_reserve_balance_before_slashing = Balances::reserved_balance(NEAL_ACCOUNT);
		assert_eq!(neal_nominator_1_reserve_balance, 100000000000000000);
		assert_eq!(neal_nominator_2_reserve_balance, 110000000000000000);
		assert_eq!(neal_reserve_balance_before_slashing, 1000000000000);

		let treasury_balance_before_slashing = Balances::free_balance(&treasury_pallet_account);
		assert_eq!(treasury_balance_before_slashing, PDEX);

		// Rotate for slashing to take place
		TheaStaking::rotate_session();
		// Make sure storage is cleaned up
		assert!(TheaStaking::commited_slashing(3).1.is_empty());

		// get alice and bob free balance after slashing
		let alice_free_balance_after_rewarding = Balances::free_balance(ALICE_ACCOUNT);
		let bob_free_balance_after_rewarding = Balances::free_balance(BOB_ACCOUNT);
		// Alice and bob free balance increases as as they reported neal's offensive behaviour
		assert_eq!(alice_free_balance_after_rewarding, 309001000000000);
		assert_eq!(bob_free_balance_after_rewarding, 309001000000000);

		// get neal's nominators as well as neal's after slashing
		let neal_nominator_1_reserve_balance_after_slashing =
			Balances::reserved_balance(NEAL_ACCOUNT_NOMINATOR_1);
		let neal_nominator_2_reserve_balance_after_slashing =
			Balances::reserved_balance(NEAL_ACCOUNT_NOMINATOR_2);
		let neal_reserve_balance_after_slashing = Balances::reserved_balance(NEAL_ACCOUNT);
		// As neal was reported for offensive behaviour, neal as well as neal nominators got slashed
		assert_eq!(neal_nominator_1_reserve_balance_after_slashing, 80000000000000000);
		assert_eq!(neal_nominator_2_reserve_balance_after_slashing, 88000000000000000);
		assert_eq!(neal_reserve_balance_after_slashing, 800000000000);

		let slashed_amount_nom_1 = neal_nominator_1_reserve_balance
			.saturating_sub(neal_nominator_1_reserve_balance_after_slashing);
		let slashed_amount_nom_2 = neal_nominator_2_reserve_balance
			.saturating_sub(neal_nominator_2_reserve_balance_after_slashing);
		let slashed_amount_neal = neal_reserve_balance_before_slashing
			.saturating_sub(neal_reserve_balance_after_slashing);

		let treasury_balance_after_getting_rewarded_from_slash =
			Balances::free_balance(&treasury_pallet_account);
		assert_eq!(treasury_balance_after_getting_rewarded_from_slash, 41581198000000000);

		let total_slashed = slashed_amount_nom_1
			.saturating_add(slashed_amount_nom_2.saturating_add(slashed_amount_neal));
		let alice_gain = alice_free_balance_after_rewarding.saturating_sub(alice_free_balance);
		let bob_gain = bob_free_balance_after_rewarding.saturating_sub(bob_free_balance);
		let pallet_gain = treasury_balance_after_getting_rewarded_from_slash
			.saturating_sub(treasury_balance_before_slashing);
		let total_gain = alice_gain.saturating_add(bob_gain.saturating_add(pallet_gain));

		assert_eq!(total_slashed, total_gain);
	});
}

#[test]
fn test_reports_under_threashold_no_slashing() {
	new_test_ext().execute_with(|| {
		misbehavior_setup_three_candidates_two_nominators();
		assert!(TheaStaking::report_offence(Origin::signed(1), 1, 3, SEVERE_OFFENCE).is_err());
		// make sure treasury is empty
		let ta = TreasuryPalletId::get().into_account_truncating();
		let treasury = Balances::free_balance(&ta);
		assert_eq!(treasury, 0);
		TheaStaking::rotate_session();
		// make sure storage cleaned up
		assert!(TheaStaking::reported_offenders(&1, SEVERE_OFFENCE).is_none());
		TheaStaking::rotate_session();
		// Now shold be ok
		TheaStaking::report_offence(Origin::signed(1), 1, 3, SEVERE_OFFENCE).unwrap();
	});
}

fn unbonding() {
	let nominator = 2u64;
	assert_ok!(TheaStaking::unbond(Origin::signed(nominator), 1_00_000_000_000));
}

fn register_nominator() {
	let nominator = 2;
	let (candidate, _, _) = get_candidate();
	Balances::mint_into(&nominator, 10_000_000_000_000u128).unwrap();
	assert_ok!(TheaStaking::bond(Origin::signed(nominator), 1_000_000_000_000u128, candidate));
}

fn register_candidate() {
	let (candidate, network_id, bls_key) = get_candidate();
	Balances::mint_into(&candidate, 10_000_000_000_000u128).unwrap();
	assert_ok!(TheaStaking::add_candidate(Origin::signed(candidate), network_id, bls_key));
}

fn register_new_candidate(candidate_id: u64, network_id: u8, bls_key: BLSPublicKey) {
	Balances::mint_into(&candidate_id, 10_000_000_000_000u128).unwrap();
	assert_ok!(TheaStaking::add_candidate(Origin::signed(candidate_id), network_id, bls_key));
}

fn insert_staking_limit() {
	let staking_limits = StakingLimits {
		mininum_relayer_stake: 1_000_000_000_000u128,
		minimum_nominator_stake: 1_000_000_000_000u128,
		maximum_nominator_per_relayer: 10,
		max_relayers: 2,
	};
	<Stakinglimits<Test>>::put(staking_limits);
}

fn get_candidate() -> (u64, u8, BLSPublicKey) {
	let candidate = 1;
	let network_id: u8 = 0;
	let bls_key = BLSPublicKey([1u8; 192]);
	(candidate, network_id, bls_key)
}
