// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//mod parachain;
use parachain_polkadex_runtime::*;
mod relay_chain;
use polkadot_parachain_primitives::primitives::{
	DmpMessageHandler, Id as ParaId, Sibling, XcmpMessageFormat, XcmpMessageHandler,
};

#[frame_support::pallet]
#[allow(unused_imports)]
pub mod mock_msg_queue {
	use super::*;
	use frame_support::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type XcmExecutor: ExecuteXcm<Self::RuntimeCall>;
	}

	// #[pallet::call]
	impl<T: Config> Pallet<T> {}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn parachain_id)]
	pub(super) type ParachainId<T: Config> = StorageValue<_, ParaId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn received_dmp)]
	/// A queue of received DMP messages
	pub(super) type ReceivedDmp<T: Config> = StorageValue<_, Vec<Xcm<T::RuntimeCall>>, ValueQuery>;

	impl<T: Config> Get<ParaId> for Pallet<T> {
		fn get() -> ParaId {
			Self::parachain_id()
		}
	}

	pub type MessageId = [u8; 32];

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		// XCMP
		/// Some XCM was executed OK.
		Success(Option<T::Hash>),
		/// Some XCM failed.
		Fail(Option<T::Hash>, XcmError),
		/// Bad XCM version used.
		BadVersion(Option<T::Hash>),
		/// Bad XCM format used.
		BadFormat(Option<T::Hash>),

		// DMP
		/// Downward message is invalid XCM.
		InvalidFormat(MessageId),
		/// Downward message is unsupported version of XCM.
		UnsupportedVersion(MessageId),
		/// Downward message executed with the given outcome.
		ExecutedDownward(MessageId, Outcome),
	}
}

use sp_runtime::BuildStorage;
use xcm::prelude::*;
use xcm_executor::traits::ConvertLocation;
use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain, TestExt};

pub const ALICE: sp_runtime::AccountId32 = sp_runtime::AccountId32::new([0u8; 32]);
pub const INITIAL_BALANCE: u128 = 1_000_000_000;

decl_test_parachain! {
	pub struct ParaA {
		Runtime = parachain_polkadex_runtime::Runtime,
		XcmpMessageHandler = mock_msg_queue::MsgQueue,
		DmpMessageHandler = mock_msg_queue::MsgQueue,
		new_ext = para_ext(1),
	}
}

decl_test_parachain! {
	pub struct ParaB {
		Runtime = parachain::Runtime,
		XcmpMessageHandler = MsgQueue,
		DmpMessageHandler = MsgQueue,
		new_ext = para_ext(2),
	}
}

decl_test_parachain! {
	pub struct ParaC {
		Runtime = parachain::Runtime,
		XcmpMessageHandler = MsgQueue,
		DmpMessageHandler = MsgQueue,
		new_ext = para_ext(3),
	}
}

decl_test_relay_chain! {
	pub struct Relay {
		Runtime = relay_chain::Runtime,
		RuntimeCall = relay_chain::RuntimeCall,
		RuntimeEvent = relay_chain::RuntimeEvent,
		XcmConfig = relay_chain::XcmConfig,
		MessageQueue = relay_chain::MessageQueue,
		System = relay_chain::System,
		new_ext = relay_ext(),
	}
}

decl_test_network! {
	pub struct MockNet {
		relay_chain = Relay,
		parachains = vec![
			(1, ParaA),
			(2, ParaB),
			(3, ParaC),
		],
	}
}

pub fn parent_account_id() -> parachain_polkadex_runtime::AccountId {
	let location = (Parent,);
	parachain_polkadex_runtime::xcm_config::LocationToAccountId::convert_location(&location.into()).unwrap()
}

pub fn child_account_id(para: u32) -> relay_chain::AccountId {
	let location = (Parachain(para),);
	relay_chain::LocationToAccountId::convert_location(&location.into()).unwrap()
}

pub fn child_account_account_id(para: u32, who: sp_runtime::AccountId32) -> relay_chain::AccountId {
	let location = (Parachain(para), AccountId32 { network: None, id: who.into() });
	relay_chain::LocationToAccountId::convert_location(&location.into()).unwrap()
}

pub fn sibling_account_account_id(para: u32, who: sp_runtime::AccountId32) -> parachain_polkadex_runtime::AccountId {
	let location = (Parent, Parachain(para), AccountId32 { network: None, id: who.into() });
	parachain_polkadex_runtime::xcm_config::LocationToAccountId::convert_location(&location.into()).unwrap()
}

pub fn parent_account_account_id(who: sp_runtime::AccountId32) -> parachain_polkadex_runtime::AccountId {
	let location = (Parent, AccountId32 { network: None, id: who.into() });
	parachain_polkadex_runtime::xcm_config::LocationToAccountId::convert_location(&location.into()).unwrap()
}

pub fn para_ext(para_id: u32) -> sp_io::TestExternalities {
	use parachain_polkadex_runtime::{MsgQueue, Runtime, System};

	let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, INITIAL_BALANCE), (parent_account_id(), INITIAL_BALANCE)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		sp_tracing::try_init_simple();
		System::set_block_number(1);
		MsgQueue::set_para_id(para_id.into());
	});
	ext
}

pub fn relay_ext() -> sp_io::TestExternalities {
	use relay_chain::{Runtime, RuntimeOrigin, System, Uniques};

	let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(ALICE, INITIAL_BALANCE),
			(child_account_id(1), INITIAL_BALANCE),
			(child_account_id(2), INITIAL_BALANCE),
			(child_account_id(3), INITIAL_BALANCE),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
		assert_eq!(Uniques::force_create(RuntimeOrigin::root(), 1, ALICE, true), Ok(()));
		assert_eq!(Uniques::mint(RuntimeOrigin::signed(ALICE), 1, 42, child_account_id(1)), Ok(()));
	});
	ext
}

pub type RelayChainPalletXcm = pallet_xcm::Pallet<relay_chain::Runtime>;
pub type ParachainPalletXcm = pallet_xcm::Pallet<parachain_polkadex_runtime::Runtime>;

#[cfg(test)]
mod tests {
	use super::*;

	use parachain_polkadex_runtime::{Balances, XcmHelper};
	use codec::{Decode, Encode};
	use frame_support::traits::fungible::Mutate;
	use frame_support::{assert_ok, weights::Weight};
	use thea_primitives::extras::ExtraData;
	use xcm::latest::QueryResponseInfo;
	use xcm_simulator::TestExt;

	// Helper function for forming buy execution message
	fn buy_execution<C>(fees: impl Into<MultiAsset>) -> Instruction<C> {
		BuyExecution { fees: fees.into(), weight_limit: Unlimited }
	}

	#[test]
	fn remote_account_ids_work() {
		child_account_account_id(1, ALICE);
		sibling_account_account_id(1, ALICE);
		parent_account_account_id(ALICE);
	}

	#[test]
	fn dmp() {
		MockNet::reset();

		let remark = parachain_polkadex_runtime::RuntimeCall::System(
			frame_system::Call::<parachain_polkadex_runtime::Runtime>::remark_with_event { remark: vec![1, 2, 3] },
		);
		Relay::execute_with(|| {
			assert_ok!(RelayChainPalletXcm::send_xcm(
				Here,
				Parachain(1),
				Xcm(vec![Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: Weight::from_parts(INITIAL_BALANCE as u64, 1024 * 1024),
					call: remark.encode().into(),
				}]),
			));
		});

		ParaA::execute_with(|| {
			use parachain_polkadex_runtime::{RuntimeEvent, System};
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::System(frame_system::Event::Remarked { .. })
			)));
		});
	}

	#[test]
	fn ump() {
		MockNet::reset();

		let remark = relay_chain::RuntimeCall::System(
			frame_system::Call::<relay_chain::Runtime>::remark_with_event { remark: vec![1, 2, 3] },
		);
		ParaA::execute_with(|| {
			assert_ok!(ParachainPalletXcm::send_xcm(
				Here,
				Parent,
				Xcm(vec![Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: Weight::from_parts(INITIAL_BALANCE as u64, 1024 * 1024),
					call: remark.encode().into(),
				}]),
			));
		});

		Relay::execute_with(|| {
			use relay_chain::{RuntimeEvent, System};
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::System(frame_system::Event::Remarked { .. })
			)));
		});
	}

	#[test]
	fn xcmp() {
		MockNet::reset();

		let remark = parachain_polkadex_runtime::RuntimeCall::System(
			frame_system::Call::<parachain_polkadex_runtime::Runtime>::remark_with_event { remark: vec![1, 2, 3] },
		);
		ParaA::execute_with(|| {
			assert_ok!(ParachainPalletXcm::send_xcm(
				Here,
				(Parent, Parachain(2)),
				Xcm(vec![Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: Weight::from_parts(INITIAL_BALANCE as u64, 1024 * 1024),
					call: remark.encode().into(),
				}]),
			));
		});

		ParaB::execute_with(|| {
			use parachain_polkadex_runtime::{RuntimeEvent, System};
			assert!(parachain_polkadex_runtime::System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::System(frame_system::Event::Remarked { .. })
			)));
		});
	}

	#[test]
	fn reserve_transfer() {
		MockNet::reset();

		let withdraw_amount = 123;

		Relay::execute_with(|| {
			assert_ok!(RelayChainPalletXcm::reserve_transfer_assets(
				relay_chain::RuntimeOrigin::signed(ALICE),
				Box::new(Parachain(1).into()),
				Box::new(AccountId32 { network: None, id: ALICE.into() }.into()),
				Box::new((Here, withdraw_amount).into()),
				0,
			));
			assert_eq!(
				relay_chain::Balances::free_balance(&child_account_id(1)),
				INITIAL_BALANCE + withdraw_amount
			);
		});

		ParaA::execute_with(|| {
			// free execution, full amount received
			use parachain_polkadex_runtime::{RuntimeEvent, System};
			assert_eq!(
				pallet_balances::Pallet::<parachain_polkadex_runtime::Runtime>::free_balance(&ALICE),
				INITIAL_BALANCE
			);
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmHelper(xcm_helper::Event::AssetDeposited(..))
			)));
		});
	}

	/// Scenario:
	/// A parachain transfers an NFT resident on the relay chain to another parachain account.
	///
	/// Asserts that the parachain accounts are updated as expected.
	#[test]
	fn withdraw_and_deposit_nft() {
		MockNet::reset();

		Relay::execute_with(|| {
			assert_eq!(relay_chain::Uniques::owner(1, 42), Some(child_account_id(1)));
		});

		ParaA::execute_with(|| {
			let message = Xcm(vec![TransferAsset {
				assets: (GeneralIndex(1), 42u32).into(),
				beneficiary: Parachain(2).into(),
			}]);
			// Send withdraw and deposit
			assert_ok!(ParachainPalletXcm::send_xcm(Here, Parent, message));
		});

		Relay::execute_with(|| {
			assert_eq!(relay_chain::Uniques::owner(1, 42), Some(child_account_id(2)));
		});
	}

	/// Scenario:
	/// A parachain transfers funds on the relay chain to another parachain account.
	///
	/// Asserts that the parachain accounts are updated as expected.
	#[test]
	fn withdraw_and_deposit() {
		MockNet::reset();

		let send_amount = 10;

		ParaA::execute_with(|| {
			let message = Xcm(vec![
				WithdrawAsset((Here, send_amount).into()),
				buy_execution((Here, send_amount)),
				DepositAsset { assets: AllCounted(1).into(), beneficiary: Parachain(2).into() },
			]);
			// Send withdraw and deposit
			assert_ok!(ParachainPalletXcm::send_xcm(Here, Parent, message.clone()));
		});

		Relay::execute_with(|| {
			assert_eq!(
				relay_chain::Balances::free_balance(child_account_id(1)),
				INITIAL_BALANCE - send_amount
			);
			assert_eq!(
				relay_chain::Balances::free_balance(child_account_id(2)),
				INITIAL_BALANCE + send_amount
			);
		});
	}

	/// Scenario:
	/// A parachain wants to be notified that a transfer worked correctly.
	/// It sends a `QueryHolding` after the deposit to get notified on success.
	///
	/// Asserts that the balances are updated correctly and the expected XCM is sent.
	#[test]
	fn query_holding() {
		MockNet::reset();

		let send_amount = 10;
		let query_id_set = 1234;

		// Send a message which fully succeeds on the relay chain
		ParaA::execute_with(|| {
			let message = Xcm(vec![
				WithdrawAsset((Here, send_amount).into()),
				buy_execution((Here, send_amount)),
				DepositAsset { assets: AllCounted(1).into(), beneficiary: Parachain(2).into() },
				ReportHolding {
					response_info: QueryResponseInfo {
						destination: Parachain(1).into(),
						query_id: query_id_set,
						max_weight: Weight::from_parts(1_000_000_000, 1024 * 1024),
					},
					assets: All.into(),
				},
			]);
			// Send withdraw and deposit with query holding
			assert_ok!(ParachainPalletXcm::send_xcm(Here, Parent, message.clone(),));
		});

		// Check that transfer was executed
		Relay::execute_with(|| {
			// Withdraw executed
			assert_eq!(
				relay_chain::Balances::free_balance(child_account_id(1)),
				INITIAL_BALANCE - send_amount
			);
			// Deposit executed
			assert_eq!(
				relay_chain::Balances::free_balance(child_account_id(2)),
				INITIAL_BALANCE + send_amount
			);
		});

		// Check that QueryResponse message was received
		ParaA::execute_with(|| {
			assert_eq!(
				parachain::MsgQueue::received_dmp(),
				vec![Xcm(vec![QueryResponse {
					query_id: query_id_set,
					response: Response::Assets(MultiAssets::new()),
					max_weight: Weight::from_parts(1_000_000_000, 1024 * 1024),
					querier: Some(Here.into()),
				}])],
			);
		});
	}

	#[test]
	fn trasnfer_pdex_token_to_non_native_chain() {
		MockNet::reset();
		ParaA::execute_with(|| {
			let destination = MultiLocation {
				parents: 1,
				interior: Junctions::X2(
					Junction::Parachain(2),
					Junction::AccountId32 { network: None, id: [1; 32] },
				),
			};
			let destination = VersionedMultiLocation::V3(destination);
			let deposit = thea_primitives::types::Withdraw {
				id: Default::default(),
				asset_id: polkadex_primitives::AssetId::Polkadex,
				amount: 2000000000000,
				destination: destination.encode(),
				fee_asset_id: None,
				fee_amount: None,
				is_blocked: false,
				extra: ExtraData::None,
			};
			let block_no = 1;
			let multlocation =
				MultiLocation { parents: 1, interior: Junctions::X1(Junction::Parachain(1)) };
			let pdex_asset_id = AssetId::Concrete(multlocation);
			assert_ok!(Balances::mint_into(&XcmHelper::get_pallet_account(), 100000000000000000));
			XcmHelper::insert_parachain_asset(
				pdex_asset_id,
				polkadex_primitives::AssetId::Polkadex,
			);
			XcmHelper::insert_pending_withdrawal(block_no, deposit.clone());
			XcmHelper::handle_new_pending_withdrawals(block_no);
		});

		ParaB::execute_with(|| {
			use parachain_polkadex_runtime::{RuntimeEvent, System};
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmHelper(xcm_helper::Event::AssetDeposited(..))
			)));
		})
	}

	#[test]
	fn test_on_initialize_with_native_asset_deposit_to_polkadex_parachain() {
		MockNet::reset();
		ParaA::execute_with(|| {
			let location =
				MultiLocation { parents: 1, interior: Junctions::X1(Junction::Parachain(1)) };
			let asset_id_ml = AssetId::Concrete(location);
			let destination = MultiLocation {
				parents: 0,
				interior: Junctions::X1(Junction::AccountId32 { network: None, id: [1; 32] }),
			};
			let destination: VersionedMultiLocation = destination.into();
			let deposit = thea_primitives::types::Withdraw {
				id: Default::default(),
				asset_id: polkadex_primitives::AssetId::Polkadex,
				amount: 2000000000000,
				destination: destination.encode(),
				fee_asset_id: None,
				fee_amount: None,
				is_blocked: false,
				extra: ExtraData::None,
			};
			assert_ok!(Balances::mint_into(&XcmHelper::get_pallet_account(), 100000000000000000));
			XcmHelper::insert_parachain_asset(asset_id_ml, polkadex_primitives::AssetId::Polkadex);
			let block_no = 1;
			XcmHelper::insert_pending_withdrawal(block_no, deposit.clone());
			XcmHelper::handle_new_pending_withdrawals(block_no);
		});
	}

	#[test]
	fn send_sibling_asset_to_non_reserve_sibling() {
		MockNet::reset();

		ParaC::execute_with(|| {
			let multlocation = MultiLocation { parents: 0, interior: Junctions::Here };
			let pdex_asset_id = AssetId::Concrete(multlocation);
			assert_ok!(Balances::mint_into(
				&XcmHelper::get_pallet_account(),
				100_000_000_000_000_000
			));
			let para_a =
				MultiLocation { parents: 1, interior: Junctions::X1(Junction::Parachain(1)) };
			let para_b =
				MultiLocation { parents: 1, interior: Junctions::X1(Junction::Parachain(2)) };
			let para_c =
				MultiLocation { parents: 1, interior: Junctions::X1(Junction::Parachain(3)) };
			assert_ok!(Balances::mint_into(
				&XcmHelper::sibling_account_converter(para_a).unwrap(),
				100_000_000_000_000_000
			));
			assert_ok!(Balances::mint_into(
				&XcmHelper::sibling_account_converter(para_b).unwrap(),
				100_000_000_000_000_000
			));
			assert_ok!(Balances::mint_into(
				&XcmHelper::sibling_account_converter(para_c).unwrap(),
				100_000_000_000_000_000
			));
			XcmHelper::insert_parachain_asset(
				pdex_asset_id,
				polkadex_primitives::AssetId::Polkadex,
			);
		});
		ParaA::execute_with(|| {
			let destination = MultiLocation {
				parents: 1,
				interior: Junctions::X2(
					Junction::Parachain(2),
					Junction::AccountId32 { network: None, id: [1; 32] },
				),
			};
			let destination = VersionedMultiLocation::V3(destination);
			let multlocation =
				MultiLocation { parents: 1, interior: Junctions::X1(Junction::Parachain(3)) };
			let non_reserve_asset_id = AssetId::Concrete(multlocation);
			let asset_id = XcmHelper::generate_asset_id_for_parachain(non_reserve_asset_id);
			let deposit = thea_primitives::types::Withdraw {
				id: Default::default(),
				asset_id,
				amount: 2000000000000,
				destination: destination.encode(),
				fee_asset_id: None,
				fee_amount: None,
				is_blocked: false,
				extra: ExtraData::None,
			};
			let block_no = 1;
			assert_ok!(Balances::mint_into(&XcmHelper::get_pallet_account(), 100000000000000000));
			XcmHelper::insert_parachain_asset(non_reserve_asset_id, asset_id);
			XcmHelper::insert_pending_withdrawal(block_no, deposit.clone());
			XcmHelper::handle_new_pending_withdrawals(block_no);
		});
		ParaC::execute_with(|| {
			let multlocation = MultiLocation { parents: 0, interior: Junctions::Here };
			let pdex_asset_id = AssetId::Concrete(multlocation);
			assert_ok!(Balances::mint_into(&XcmHelper::get_pallet_account(), 100000000000000000));
			XcmHelper::insert_parachain_asset(
				pdex_asset_id,
				polkadex_primitives::AssetId::Polkadex,
			);
			use parachain_polkadex_runtime::{RuntimeEvent, System};
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmHelper(xcm_helper::Event::SiblingDeposit(..))
			)));
		});
		ParaB::execute_with(|| {
			use parachain_polkadex_runtime::{RuntimeEvent, System};
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmHelper(xcm_helper::Event::AssetDeposited(..))
			)));
		})
	}

	#[test]
	fn test_with_thea_payload() {
		MockNet::reset();
		ParaA::execute_with(|| {
			let hex = hex::decode("0400000000c9104ef90d846adb8cd6727f57c8e46d010010a5d4e800000000000000000000007003010200511f03007b9a3a0bd813c61006d94c7206137052f1cd6ce100000000").unwrap();
			let payload: Vec<thea_primitives::types::Withdraw> =
				Decode::decode(&mut &hex[..]).unwrap();
			let block_no = 1;
			let multlocation =
				MultiLocation { parents: 1, interior: Junctions::X1(Junction::Parachain(1)) };
			let pdex_asset_id = AssetId::Concrete(multlocation);
			XcmHelper::insert_parachain_asset(
				pdex_asset_id,
				polkadex_primitives::AssetId::Polkadex,
			);
			XcmHelper::insert_pending_withdrawal(block_no, payload.first().unwrap().clone());
			XcmHelper::handle_new_pending_withdrawals(block_no);
		})
	}
}
