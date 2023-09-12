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

use super::{
	AccountId, Balances, ParachainInfo, ParachainSystem, PolkadotXcm, Runtime, RuntimeCall,
	RuntimeEvent, RuntimeOrigin, WeightToFee, XcmpQueue,
};
use crate::{AllPalletsWithSystem, Balance, PolkadexAssetid, XcmHelper};
use core::marker::PhantomData;
use frame_support::{
	match_types, parameter_types,
	traits::{Contains, Everything, Nothing},
	weights::WeightToFee as WeightToFeeT,
};
use frame_system::EnsureRoot;

use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key};
use orml_xcm_support::MultiNativeAsset;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use polkadot_runtime_common::impls::ToAuthor;
use sp_core::{ConstU32, Get};
use sp_runtime::{traits::Convert, SaturatedConversion};
use xcm::latest::{prelude::*, Weight as XCMWeight, Weight};
use xcm_builder::{
	AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, CurrencyAdapter, EnsureXcmOrigin, FixedWeightBounds,
	IsConcrete, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative,
	SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
	SovereignSignedViaLocation, TakeRevenue, TakeWeightCredit, UsingComponents,
};
use xcm_executor::{
	traits::{WeightTrader, WithOriginFilter},
	Assets, XcmExecutor,
};
use xcm_helper::{AssetIdConverter, WhitelistedTokenHandler};

parameter_types! {
	pub const RelayLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Polkadot;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
	pub PdexLocation: MultiLocation = Here.into();
	pub UniversalLocation: InteriorMultiLocation = X2(GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into()));


}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type LocalAssetTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<RelayLocation>,
	// Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports.
	(),
>;

// Not using it for now. Saved for future.
pub struct SafeCallFilter;
impl SafeCallFilter {
	// 1. RuntimeCall::EVM(..) & RuntimeCall::Ethereum(..) have to be prohibited since we cannot
	//    measure PoV size properly
	// 2. RuntimeCall::Contracts(..) can be allowed, but it hasn't been tested properly yet.

	/// Checks whether the base (non-composite) call is allowed to be executed via `Transact` XCM
	/// instruction.
	pub fn allow_base_call(call: &RuntimeCall) -> bool {
		matches!(
			call,
			RuntimeCall::System(..) |
				RuntimeCall::Balances(..) |
				RuntimeCall::Assets(..) |
				RuntimeCall::PolkadotXcm(..) |
				RuntimeCall::Session(..)
		)
	}
	/// Checks whether composite call is allowed to be executed via `Transact` XCM instruction.
	///
	/// Each composite call's subcalls are checked against base call filter. No nesting of composite
	/// calls is allowed.
	pub fn allow_composite_call(_call: &RuntimeCall) -> bool {
		false
	}
}

impl Contains<RuntimeCall> for SafeCallFilter {
	fn contains(call: &RuntimeCall) -> bool {
		Self::allow_base_call(call) || Self::allow_composite_call(call)
	}
}

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	// Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `RuntimeOrigin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
	pub const UnitWeightCost: XCMWeight = XCMWeight::from_parts(200_000_000, 0);
	pub const MaxInstructions: u32 = 100;
}

match_types! {
	pub type ParentOrParentsExecutivePlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Executive, .. }) }
	};
}

pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
	// Expected responses are OK.
	AllowKnownQueryResponses<PolkadotXcm>,
	// Subscriptions for version tracking are OK.
	AllowSubscriptionsFrom<Everything>,
);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	// How to withdraw and deposit an asset.
	type AssetTransactor = XcmHelper;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = MultiNativeAsset<AbsoluteReserveProvider>;
	// Teleporting is disabled.
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type Trader = (
		// If the XCM message is paying the fees in PDEX ( the native ) then
		// it will go to the author of the block as rewards
		UsingComponents<WeightToFee, PdexLocation, AccountId, Balances, ToAuthor<Runtime>>,
		ForeignAssetFeeHandler<
			//TODO: Should we go for FixedRateOfForeignAsset
			WeightToFee,
			RevenueCollector,
			XcmHelper,
			XcmHelper,
		>,
	);
	type ResponseHandler = PolkadotXcm;
	type AssetTrap = PolkadotXcm;
	type AssetLocker = ();
	type AssetExchanger = ();
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = ConstU32<64>;
	type FeeManager = ();
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = WithOriginFilter<SafeCallFilter>;
	type SafeCallFilter = Everything; //Note: All kind of ext can be accessed through XCM
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Nothing;
	// ^ Disable dispatchable execute on the XCM pallet.
	// Needs to be `Everything` for local testing.
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Nothing;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	// ^ Override for AdvertisedXcmVersion default
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type AdminOrigin = EnsureRoot<AccountId>;
	type TrustedLockers = ();
	type SovereignAccountOf = LocationToAccountId;
	type MaxLockers = ConstU32<8>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	type WeightInfo = pallet_xcm::TestWeightInfo;
	#[cfg(feature = "polkadex-parachain-benchmarks")]
	type ReachableDest = ();
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

pub struct AccountIdToMultiLocation;
impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
	fn convert(account: AccountId) -> MultiLocation {
		X1(AccountId32 { network: None, id: account.into() }).into()
	}
}

parameter_types! {
	pub SelfLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(ParachainInfo::get().into())));
	pub BaseXcmWeight: Weight =  XCMWeight::from_parts(100_000_000, 0);
	pub const MaxAssetsForTransfer: usize = 2;
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: MultiLocation| -> Option<u128> {
		Some(1u128)
	};
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = u128;
	type CurrencyIdConvert = XcmHelper;
	type AccountIdToMultiLocation = AccountIdToMultiLocation;
	type SelfLocation = SelfLocation;
	type MinXcmFee = ParachainMinFee;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type MultiLocationsFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = BaseXcmWeight;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type ReserveProvider = AbsoluteReserveProvider;
	type UniversalLocation = UniversalLocation;
}

pub struct ForeignAssetFeeHandler<T, R, AC, WH>
where
	T: WeightToFeeT<Balance = u128>,
	R: TakeRevenue,
	AC: AssetIdConverter,
	WH: WhitelistedTokenHandler,
{
	/// Total used weight
	weight: Weight,
	/// Total consumed assets
	consumed: u128,
	/// Asset Id (as MultiLocation) and units per second for payment
	asset_location_and_units_per_second: Option<(MultiLocation, u128)>,
	_pd: PhantomData<(T, R, AC, WH)>,
}

impl<T, R, AC, WH> WeightTrader for ForeignAssetFeeHandler<T, R, AC, WH>
where
	T: WeightToFeeT<Balance = u128>,
	R: TakeRevenue,
	AC: AssetIdConverter,
	WH: WhitelistedTokenHandler,
{
	fn new() -> Self {
		Self {
			weight: Weight::zero(),
			consumed: 0,
			asset_location_and_units_per_second: None,
			_pd: PhantomData,
		}
	}

	/// NOTE: If the token is allowlisted by AMM pallet ( probably using governance )
	/// then it will be allowed to execute for free even if the pool is not there.
	/// If pool is not there and token is not present in allowlisted then it will be rejected.
	fn buy_weight(
		&mut self,
		weight: Weight,
		payment: Assets,
	) -> sp_std::result::Result<Assets, XcmError> {
		let _fee_in_native_token = T::weight_to_fee(&weight);
		let payment_asset = payment.fungible_assets_iter().next().ok_or(XcmError::Trap(1000))?;
		if let AssetId::Concrete(location) = payment_asset.id {
			let foreign_currency_asset_id =
				AC::convert_location_to_asset_id(location).ok_or(XcmError::Trap(1001))?;
			let _path = [PolkadexAssetid::get(), foreign_currency_asset_id];
			let (unused, expected_fee_in_foreign_currency) =
				if WH::check_whitelisted_token(foreign_currency_asset_id) {
					(payment, 0u128)
				} else {
					return Err(XcmError::Trap(1004))
				};
			self.weight = self.weight.saturating_add(weight);
			if let Some((old_asset_location, _)) = self.asset_location_and_units_per_second {
				if old_asset_location == location {
					self.consumed = self
						.consumed
						.saturating_add((expected_fee_in_foreign_currency).saturated_into());
				}
			} else {
				self.consumed = self
					.consumed
					.saturating_add((expected_fee_in_foreign_currency).saturated_into());
				self.asset_location_and_units_per_second = Some((location, 0));
			}
			Ok(unused)
		} else {
			Err(XcmError::Trap(1005))
		}
	}
}

impl<T, R, AC, WH> Drop for ForeignAssetFeeHandler<T, R, AC, WH>
where
	T: WeightToFeeT<Balance = u128>,
	R: TakeRevenue,
	AC: AssetIdConverter,
	WH: WhitelistedTokenHandler,
{
	fn drop(&mut self) {
		if let Some((asset_location, _)) = self.asset_location_and_units_per_second {
			if self.consumed > 0 {
				R::take_revenue((asset_location, self.consumed).into());
			}
		}
	}
}

pub struct TypeConv;
impl<Source: TryFrom<Dest> + Clone, Dest: TryFrom<Source> + Clone>
	xcm_executor::traits::Convert<Source, Dest> for TypeConv
{
	fn convert(value: Source) -> Result<Dest, Source> {
		Dest::try_from(value.clone()).map_err(|_| value)
	}
}

pub struct RevenueCollector;

impl TakeRevenue for RevenueCollector {
	fn take_revenue(_revenue: MultiAsset) {}
}
