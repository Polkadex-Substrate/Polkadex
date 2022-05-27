// This file is part of Polkadex.

// Copyright (C) 2020-2022 Polkadex oü.
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
use crate as thea;
use frame_support::{parameter_types, traits::ConstU32};
use frame_system as system;
use pallet_session::{historical as pallet_session_historical, SessionHandler};
use sp_core::H256;
use sp_runtime::{
	key_types::DUMMY,
	testing::{Header, TestXt, UintAuthorityId},
	traits::{
		BlakeTwo256, ConvertInto, Extrinsic as ExtrinsicT, IdentityLookup, OpaqueKeys, Verify,
	},
};
use sp_staking::offence::OffenceError;
use std::{cell::RefCell, collections::BTreeMap};
use thea_primitives::{traits::HandleSignedPayloadTrait, AccountId, AuthorityId, Signature};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Thea: thea::{Pallet, Call, Storage, Event<T>, ValidateUnsigned},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		Historical: pallet_session_historical::{Pallet}
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

type Extrinsic = TestXt<Call, ()>;

impl system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> system::offchain::SendTransactionTypes<LocalCall> for Test
where
	Call: From<LocalCall>,
{
	type OverarchingCall = Call;
	type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
	Call: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call,
		_public: <Signature as Verify>::Signer,
		_account: AccountId,
		nonce: u64,
	) -> Option<(Call, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
		Some((call, (nonce, ())))
	}
}

thread_local! {
	pub static VALIDATORS: RefCell<Option<Vec<AccountId32>>> = RefCell::new(Some(vec![
		AccountId32::new([0;32]),
		AccountId32::new([1;32]),
		AccountId32::new([2;32]),
	]));
}

pub struct TestSessionManager;
impl pallet_session::SessionManager<AccountId32> for TestSessionManager {
	fn new_session(_new_index: SessionIndex) -> Option<Vec<AccountId32>> {
		VALIDATORS.with(|l| l.borrow_mut().take())
	}
	fn end_session(_: SessionIndex) {}
	fn start_session(_: SessionIndex) {}
}

impl pallet_session::historical::SessionManager<AccountId32, AccountId32> for TestSessionManager {
	fn new_session(_new_index: SessionIndex) -> Option<Vec<(AccountId32, AccountId32)>> {
		VALIDATORS.with(|l| {
			l.borrow_mut()
				.take()
				.map(|validators| validators.iter().map(|v| (v.clone(), v.clone())).collect())
		})
	}
	fn end_session(_: SessionIndex) {}
	fn start_session(_: SessionIndex) {}
}

pub struct DummyEthereumHandler;
impl HandleSignedPayloadTrait for DummyEthereumHandler {
	fn handle_signed_payload(payload: SignedTheaPayload) {
		assert_eq!(payload.payload.network, Network::ETHEREUM)
	}
}
thread_local! {
	pub static AUTHORITIES: RefCell<Vec<UintAuthorityId>> =
		RefCell::new(vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)]);
	pub static FORCE_SESSION_END: RefCell<bool> = RefCell::new(false);
	pub static SESSION_LENGTH: RefCell<u64> = RefCell::new(2);
	pub static SESSION_CHANGED: RefCell<bool> = RefCell::new(false);
	pub static TEST_SESSION_CHANGED: RefCell<bool> = RefCell::new(false);
	pub static DISABLED: RefCell<bool> = RefCell::new(false);
	// Stores if `on_before_session_end` was called
	pub static BEFORE_SESSION_END_CALLED: RefCell<bool> = RefCell::new(false);
	pub static VALIDATOR_ACCOUNTS: RefCell<BTreeMap<u64, u64>> = RefCell::new(BTreeMap::new());
}
pub struct TestSessionHandler;
impl SessionHandler<AccountId32> for TestSessionHandler {
	const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[UintAuthorityId::ID];
	fn on_genesis_session<T: OpaqueKeys>(_validators: &[(AccountId32, T)]) {}
	fn on_new_session<T: OpaqueKeys>(
		changed: bool,
		validators: &[(AccountId32, T)],
		_queued_validators: &[(AccountId32, T)],
	) {
		SESSION_CHANGED.with(|l| *l.borrow_mut() = changed);
		AUTHORITIES.with(|l| {
			*l.borrow_mut() = validators
				.iter()
				.map(|(_, id)| id.get::<UintAuthorityId>(DUMMY).unwrap_or_default())
				.collect()
		});
	}
	fn on_disabled(_validator_index: u32) {
		DISABLED.with(|l| *l.borrow_mut() = true)
	}
	fn on_before_session_ending() {
		BEFORE_SESSION_END_CALLED.with(|b| *b.borrow_mut() = true);
	}
}

// TODO: need to have config for offense pallet
type IdentificationTuple = (AccountId32, AccountId32);
type Offence = crate::UnresponsivenessOffence<IdentificationTuple>;

thread_local! {
	pub static OFFENCES: RefCell<Vec<(Vec<AccountId32>, Offence)>> = RefCell::new(vec![]);
}

pub struct OffenceHandler;
impl ReportOffence<AccountId32, IdentificationTuple, Offence> for OffenceHandler {
	fn report_offence(reporters: Vec<AccountId32>, offence: Offence) -> Result<(), OffenceError> {
		OFFENCES.with(|l| l.borrow_mut().push((reporters, offence)));
		Ok(())
	}

	fn is_known_offence(_offenders: &[IdentificationTuple], _time_slot: &SessionIndex) -> bool {
		false
	}
}

parameter_types! {
	pub const Period: u64 = 1;
	pub const Offset: u64 = 0;
}

impl pallet_session::Config for Test {
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, TestSessionManager>;
	type SessionHandler = TestSessionHandler;
	type ValidatorId = AccountId32;
	type ValidatorIdOf = ConvertInto;
	type Keys = UintAuthorityId;
	type Event = Event;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type WeightInfo = ();
}

impl pallet_session::historical::Config for Test {
	type FullIdentification = AccountId32;
	type FullIdentificationOf = ConvertInto;
}

parameter_types! {
	pub const PayloadLimit: u32 = 50;
}

impl thea::Config for Test {
	type Event = Event;
	type TheaId = AuthorityId;
	type EthereumHandler = DummyEthereumHandler;
	type PayloadLimit = PayloadLimit;
	type TheaWeightInfo = thea::WeightInfo<Test>;
	type ReportMisbehaviour = OffenceHandler;
	type ValidatorSet = Historical;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap_or(Default::default());
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
