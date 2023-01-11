use std::collections::{BTreeSet, HashSet};
use crate::{mock::*, Error, Event, Exposure, Stakinglimits, IndividualExposure};
use frame_support::{assert_noop, assert_ok};
use frame_support::traits::fungible::Mutate;
use frame_support::traits::TheseExcept;
use crate as thea_staking;
use crate::session::{StakingLimits, UnlockChunk};


//TODO Things to test
//1. Bound -> Nominate -> Bound
//2/ Bound : CandidateNotFound


#[test]
fn test_add_candidate_with_valid_inputs_returns_ok() {
    new_test_ext().execute_with(|| {
        let candidate = 1;
        let network_id: u8 = 0;
        let bls_key = [1u8;65];
        Balances::mint_into(&candidate, 10_000_000_000_000u128);
        assert_ok!(TheaStaking::add_candidate(Origin::signed(candidate), network_id, bls_key));
        assert_eq!(Balances::free_balance(&candidate), 9_000_000_000_000);
        assert_eq!(Balances::reserved_balance(&candidate), 1_000_000_000_000);
        let exposure = Exposure {
            score: 1000,
            total: 1_000_000_000_000,
            bls_pub_key: bls_key,
            stakers: Default::default()
        };
        assert_eq!(TheaStaking::candidates(network_id ,candidate), Some(exposure));
        assert_eq!(TheaStaking::candidates_to_network(candidate), Some(network_id));
    });
}

#[test]
fn test_add_candidate_with_already_registered_candidate_returns_CandidateAlreadyRegistered_error() {
    new_test_ext().execute_with(|| {
        let candidate = 1;
        let network_id: u8 = 0;
        let bls_key = [1u8;65];
        Balances::mint_into(&candidate, 10_000_000_000_000u128);
        assert_ok!(TheaStaking::add_candidate(Origin::signed(candidate), network_id, bls_key));
        assert_noop!(TheaStaking::add_candidate(Origin::signed(candidate), network_id, bls_key), Error::<Test>::CandidateAlreadyRegistered);
    });
}

#[test]
fn test_add_candidate_with_low_free_balance_returns_low_balance_error() {
    new_test_ext().execute_with(|| {
        let candidate = 1;
        let network_id: u8 = 0;
        let bls_key = [1u8;65];
        Balances::mint_into(&candidate, 10_000_000_000u128);
        assert_noop!(TheaStaking::add_candidate(Origin::signed(candidate), network_id, bls_key), pallet_balances::Error::<Test>::InsufficientBalance);
    });
}

#[test]
fn test_bound_with_valid_arguments_first_time_returns_ok() {
    new_test_ext().execute_with(|| {
        register_candidate();
        insert_staking_limit();
        // Give some Balance to Nominator
        let nominator = 2;
        Balances::mint_into(&nominator, 10_000_000_000_000u128);
        assert_ok!(TheaStaking::bond(Origin::signed(nominator), 1_000_000_000_000u128));
        let individual_exposure = IndividualExposure {
            who: nominator,
            value: 1_000_000_000_000u128,
            backing: None,
            unlocking: vec![]
        };
        assert_eq!(TheaStaking::stakers(nominator), Some(individual_exposure));
    });
}

#[test]
fn test_bound_with_low_nominators_balance_returns_StakingLimitsError() {
    new_test_ext().execute_with(|| {
        register_candidate();
        insert_staking_limit();
        let nominator = 2;
        assert_noop!(TheaStaking::bond(Origin::signed(nominator), 1_000_000_000_00u128), Error::<Test>::StakingLimitsError);
    });
}

#[test]
fn test_bound_with_low_nominators_balance_return_InsufficientBalance() {
    new_test_ext().execute_with(|| {
        register_candidate();
        insert_staking_limit();
        let nominator = 2;
        assert_noop!(TheaStaking::bond(Origin::signed(nominator), 1_000_000_000_000u128), pallet_balances::Error::<Test>::InsufficientBalance);
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
        assert_ok!(TheaStaking::nominate(Origin::signed(nominator), candidate));
        let mut stakers: BTreeSet<u64> = BTreeSet::new();
        stakers.insert(nominator);
        let exposure = Exposure {
            score: 1000,
            total: 2_000_000_000_000,
            bls_pub_key: bls_key,
            stakers
        };
        assert_eq!(TheaStaking::candidates(network_id, candidate), Some(exposure));
        let nominator_exposure = IndividualExposure {
            who: nominator,
            value: 1_000_000_000_000u128,
            backing: Some((network_id, candidate)),
            unlocking: vec![]
        };
        assert_eq!(TheaStaking::stakers(nominator), Some(nominator_exposure));
    });
}

#[test]
fn test_nominate_with_invalid_nominator_returns_StakerNotFound() {
    new_test_ext().execute_with(|| {
        let nominator = 2;
        let candidate = 1;
        assert_noop!(TheaStaking::nominate(Origin::signed(nominator), candidate), Error::<Test>::StakerNotFound);
    })
}

#[test]
fn test_nominate_with_already_staked_relayer_returns_StakerAlreadyNominating() {
    new_test_ext().execute_with(|| {
        register_candidate();
        insert_staking_limit();
        register_nominator();
        let (candidate, ..) = get_candidate();
        let nominator = 2;
        assert_ok!(TheaStaking::nominate(Origin::signed(nominator), candidate));
        assert_noop!(TheaStaking::nominate(Origin::signed(nominator), candidate), Error::<Test>::StakerAlreadyNominating);
    })
}

#[test]
fn test_nominate_with_wrong_candidate_returns_CandidateNotFound() {
    new_test_ext().execute_with(|| {
        insert_staking_limit();
        register_nominator();
        let (candidate, ..) = get_candidate();
        let nominator = 2;
        assert_noop!(TheaStaking::nominate(Origin::signed(nominator), candidate), Error::<Test>::CandidateNotFound);
    });
}

// If Nominator tries to bound more with nominating anyone. Then Nominator will loose tokens
// This test should pass.
#[ignore]
#[test]
fn test_bound_with_valid_arguments_second_time_returns_ok() {
    new_test_ext().execute_with(|| {
        register_candidate();
        insert_staking_limit();
        // Give some Balance to Nominator
        let nominator = 2;
        Balances::mint_into(&nominator, 10_000_000_000_000u128);
        assert_ok!(TheaStaking::bond(Origin::signed(nominator), 1_000_000_000_000u128));
        assert_ok!(TheaStaking::bond(Origin::signed(nominator), 1_000_000_000_000u128));
        let individual_exposure = IndividualExposure {
            who: nominator,
            value: 2_000_000_000_000u128,
            backing: None,
            unlocking: vec![]
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
        assert_ok!(TheaStaking::nominate(Origin::signed(nominator), candidate));
        assert_ok!(TheaStaking::unbond(Origin::signed(nominator), 1_00_000_000_000));
        let mut stakers: BTreeSet<u64> = BTreeSet::new();
        stakers.insert(nominator);
        let relayer_exposure = Exposure {
            score: 1000,
            total: 1_900_000_000_000u128,
            bls_pub_key: bls_key,
            stakers: stakers
        };
        assert_eq!(TheaStaking::candidates(network, candidate), Some(relayer_exposure));
    })
}

#[test]
fn test_unbond_with_unregistered_nominator_returns_StakerNotFound_error() {
    new_test_ext().execute_with(|| {
        register_candidate();
        insert_staking_limit();
        let nominator = 2u64;
        assert_noop!(TheaStaking::unbond(Origin::signed(nominator), 1_000_000_000_000), Error::<Test>::StakerNotFound);
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
            backing: None,
            unlocking: vec![unlocking_chunk]
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
        assert_ok!(TheaStaking::withdraw_unbonded(Origin::signed(nominator)));
    })
}

#[test]
fn test_withdraw_unbouded_with_unregistered_nominator_returns_error() {
    new_test_ext().execute_with(|| {
        let nominator = 2u64;
        assert_noop!(TheaStaking::withdraw_unbonded(Origin::signed(nominator)), Error::<Test>::CandidateNotFound);
    })
}

#[test]
fn test_remove_candidate_with_right_arguments_returns_ok() {
    new_test_ext().execute_with(|| {
        register_candidate();
        let (candidate,network,..) = get_candidate();
        assert_ok!(TheaStaking::remove_candidate(Origin::signed(candidate), network));
    })
}

#[test]
fn test_remove_candidate_with_wrong_netowork_id_returns_error() {
    new_test_ext().execute_with(|| {
        register_candidate();
        let (candidate,..) = get_candidate();
        let wrong_network_id = 5;
        assert_noop!(TheaStaking::remove_candidate(Origin::signed(candidate),wrong_network_id), Error::<Test>::CandidateNotFound);
    })
}

#[test]
fn test_remove_candidate_with_unregistered_nominator_returns_error() {
    new_test_ext().execute_with(|| {
        let (candidate, network_id, ..) = get_candidate();
        assert_noop!(TheaStaking::remove_candidate(Origin::signed(candidate), network_id), Error::<Test>::CandidateNotFound);
    })
}

#[ignore]
#[test]
fn test_unbond_with_amount_more_than_staked_amount_returns_error() {}

#[ignore]
#[test]
fn test_unbond_with_amount_equal_to_staked_amount_returns_ok() {}

fn unbonding() {
    let nominator = 2u64;
    assert_ok!(TheaStaking::unbond(Origin::signed(nominator), 1_00_000_000_000));
}

fn register_nominator() {
    let nominator = 2;
    Balances::mint_into(&nominator, 10_000_000_000_000u128);
    assert_ok!(TheaStaking::bond(Origin::signed(nominator), 1_000_000_000_000u128));
}

fn register_candidate() {
    let (candidate, network_id, bls_key) = get_candidate();
    Balances::mint_into(&candidate, 10_000_000_000_000u128);
    assert_ok!(TheaStaking::add_candidate(Origin::signed(candidate), network_id, bls_key));
}

fn insert_staking_limit() {
    let staking_limits = StakingLimits {
        mininum_relayer_stake: 1_000_000_000_000u128,
        minimum_nominator_stake: 1_000_000_000_000u128,
        maximum_nominator_per_relayer: 10,
        max_relayers: 10
    };
    <Stakinglimits<Test>>::put(staking_limits);
}

fn get_candidate() -> (u64, u8, [u8;65]) {
    let candidate = 1;
    let network_id: u8 = 0;
    let bls_key = [1u8;65];
    (candidate, network_id, bls_key)
}
