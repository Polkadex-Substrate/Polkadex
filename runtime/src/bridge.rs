use crate::{
	Index, MaxAuthorities, Runtime, RuntimeBlockLength, RuntimeBlockWeights, RuntimeEvent,
	TheaExecutor, MAXIMUM_BLOCK_WEIGHT, NORMAL_DISPATCH_RATIO,
};
use bp_header_chain::ChainWithGrandpa;
use bp_messages::{
	target_chain::{DispatchMessage, MessageDispatch},
	ChainWithMessages, MessageNonce,
};
use bp_parachains::SingleParaStoredHeaderDataBuilder;
use bp_runtime::{messages::MessageDispatchResult, Chain, ChainId, Parachain};
use frame_support::{
	dispatch::DispatchClass, parameter_types, traits::ConstU32, weights::Weight, RuntimeDebug,
};
use frame_system::limits::{BlockLength, BlockWeights};
use parity_scale_codec::{Decode, Error};
use polkadex_primitives::{AccountId, Balance, BlockNumber, Hash, Signature};
use sp_core::crypto::AccountId32;
use sp_runtime::{generic, traits::BlakeTwo256};
use sp_std::vec::Vec;
use sp_version::StateVersion;
use thea_primitives::{
	types::{Deposit, Withdraw},
	Network,
};
// This is polkadot related DAYS and not Polkadex
pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub type PolkadotGrandpaInstance = ();
impl pallet_bridge_grandpa::Config<PolkadotGrandpaInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BridgedChain = Polkadot;
	type MaxFreeMandatoryHeadersPerBlock = ConstU32<4>;
	type HeadersToKeep = ConstU32<{ DAYS }>;
	type WeightInfo = pallet_bridge_grandpa::weights::BridgeWeight<Runtime>;
}

/// Maximal size of encoded `bp_parachains::ParaStoredHeaderData` structure among all Polkadot
/// parachains.
///
/// It includes the block number and state root, so it shall be near 40 bytes, but let's have some
/// reserve.
pub const MAX_NESTED_PARACHAIN_HEAD_DATA_SIZE: u32 = 128;
/// Name of the parachains pallet in the Polkadot runtime.
pub const PARAS_PALLET_NAME: &'static str = "Paras";
parameter_types! {
	pub const PolkadotParasPalletName: &'static str = PARAS_PALLET_NAME;
	pub const MaxPolkadexParaHeadDataSize: u32 = MAX_NESTED_PARACHAIN_HEAD_DATA_SIZE;
}

impl pallet_bridge_parachains::Config<PolkadexParachainInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_bridge_parachains::weights::BridgeWeight<Runtime>;
	type BridgesGrandpaPalletInstance = PolkadotGrandpaInstance;
	type ParasPalletName = PolkadotParasPalletName;
	type ParaStoredHeaderDataBuilder = SingleParaStoredHeaderDataBuilder<PolkadexParachain>;
	type HeadsToKeep = ConstU32<1024>;
	type MaxParaHeadDataSize = MaxPolkadexParaHeadDataSize;
}

/// Instance of the messages pallet used to relay messages to/from Rialto chain.
pub type PolkadexParachainInstance = ();
impl pallet_bridge_messages::Config<PolkadexParachainInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_bridge_messages::weights::BridgeWeight<Runtime>;
	type ThisChain = Mainnet;
	type BridgedChain = PolkadexParachain;
	type BridgedHeaderChain = pallet_bridge_parachains::ParachainHeaders<
		Runtime,
		PolkadexParachainInstance,
		PolkadexParachain,
	>;
	type OutboundPayload = sp_std::vec::Vec<u8>;
	type InboundPayload = Vec<u8>;
	type DeliveryPayments = ();
	type DeliveryConfirmationPayments = ();
	type MessageDispatch = IncomingMessagesHandler;
}

pub struct IncomingMessagesHandler;
impl MessageDispatch for IncomingMessagesHandler {
	type DispatchPayload = Vec<u8>;
	type DispatchLevelResult = ();

	fn dispatch_weight(_: &mut DispatchMessage<Self::DispatchPayload>) -> Weight {
		Weight::zero() // TODO: Update this weight
	}

	fn dispatch(
		message: DispatchMessage<Self::DispatchPayload>,
	) -> MessageDispatchResult<Self::DispatchLevelResult> {
		match message.data.payload {
			Ok(mut encoded_deposits) => {
				let deposits: Vec<Deposit<AccountId>> =
					match Decode::decode(&mut &encoded_deposits[..]) {
						Ok(deposits) => deposits,
						Err(_) => {
							return MessageDispatchResult {
								unspent_weight: Default::default(), // TODO: Update this weight
								dispatch_level_result: (),
							}
						},
					};
				TheaExecutor::do_deposit(1, deposits);
			},
			Err(_) =>
				return MessageDispatchResult {
					unspent_weight: Default::default(), // TODO: Update this weight
					dispatch_level_result: (),
				},
		}

		MessageDispatchResult {
			unspent_weight: Default::default(), // TODO: Update this weight
			dispatch_level_result: (),
		}
	}
}

/// Underlying chain of `ThisChain`.
pub struct Mainnet;
impl Chain for Mainnet {
	const ID: ChainId = *b"main";

	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hasher = BlakeTwo256;
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	type AccountId = AccountId;
	type Balance = Balance;
	type Index = Index;
	type Signature = Signature;

	const STATE_VERSION: StateVersion = StateVersion::V0;

	fn max_extrinsic_size() -> u32 {
		*RuntimeBlockLength::get().max.get(DispatchClass::Normal)
	}

	fn max_extrinsic_weight() -> Weight {
		RuntimeBlockWeights::get()
			.get(DispatchClass::Normal)
			.max_extrinsic
			.unwrap_or(Weight::MAX)
	}
}

impl ChainWithMessages for Mainnet {
	const WITH_CHAIN_MESSAGES_PALLET_NAME: &'static str = "SubMessages";

	const MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX: MessageNonce = 16;
	const MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX: MessageNonce = 1000;
}

impl ChainWithGrandpa for Mainnet {
	const WITH_CHAIN_GRANDPA_PALLET_NAME: &'static str = WITH_MAINNET_GRANDPA_PALLET_NAME;
	const MAX_AUTHORITIES_COUNT: u32 = MaxAuthorities::get();
	const REASONABLE_HEADERS_IN_JUSTIFICATON_ANCESTRY: u32 =
		REASONABLE_HEADERS_IN_JUSTIFICATON_ANCESTRY;
	const MAX_HEADER_SIZE: u32 = MAX_HEADER_SIZE;
	const AVERAGE_HEADER_SIZE_IN_JUSTIFICATION: u32 = AVERAGE_HEADER_SIZE_IN_JUSTIFICATION;
}

/// Polkadex parachain.
#[derive(RuntimeDebug)]
pub struct PolkadexParachain;
impl Chain for PolkadexParachain {
	const ID: ChainId = *b"para";

	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hasher = BlakeTwo256;
	type Header = generic::Header<BlockNumber, BlakeTwo256>;

	type AccountId = AccountId;
	type Balance = Balance;
	type Index = Index;
	type Signature = Signature;

	const STATE_VERSION: StateVersion = StateVersion::V1;

	fn max_extrinsic_size() -> u32 {
		*RuntimeBlockLength::get().max.get(DispatchClass::Normal)
	}

	fn max_extrinsic_weight() -> Weight {
		RuntimeBlockWeights::get()
			.get(DispatchClass::Normal)
			.max_extrinsic
			.unwrap_or(Weight::MAX)
	}
}

impl Parachain for PolkadexParachain {
	const PARACHAIN_ID: u32 = 2040;
}

impl ChainWithMessages for PolkadexParachain {
	const WITH_CHAIN_MESSAGES_PALLET_NAME: &'static str = "SubMessages";

	const MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX: MessageNonce = 16;
	const MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX: MessageNonce = 1000;
}

/// Maximal extrinsic size at the `BridgedChain`.
pub const BRIDGED_CHAIN_MAX_EXTRINSIC_SIZE: u32 = 1024; // TODO: Check if this is enough

/// Name of the GRANDPA pallet instance that is deployed at Polkadex Mainnet
pub const WITH_MAINNET_GRANDPA_PALLET_NAME: &str = "Grandpa";
pub const WITH_POLKADOT_GRANDPA_PALLET_NAME: &str = "Grandpa";

/// Reasonable number of headers in the `votes_ancestries` on Mainnet.
///
/// See [`bp-header-chain::ChainWithGrandpa`] for more details.
pub const REASONABLE_HEADERS_IN_JUSTIFICATON_ANCESTRY: u32 = 8;

/// Approximate average header size in `votes_ancestries` field of justification on Mainnet.
///
/// See [`bp-header-chain::ChainWithGrandpa`] for more details.
pub const AVERAGE_HEADER_SIZE_IN_JUSTIFICATION: u32 = 256;

/// Approximate maximal header size on Mainnet
///
/// We expect maximal header to have digest item with the new authorities set for every consensus
/// engine (GRANDPA, Babe, BEEFY, ...) - so we multiply it by 3. And also
/// `AVERAGE_HEADER_SIZE_IN_JUSTIFICATION` bytes for other stuff.
///
/// See [`bp-header-chain::ChainWithGrandpa`] for more details.
pub const MAX_HEADER_SIZE: u32 = MaxAuthorities::get()
	.saturating_mul(3)
	.saturating_add(AVERAGE_HEADER_SIZE_IN_JUSTIFICATION);

/// Rialto chain.
#[derive(RuntimeDebug)]
pub struct Polkadot;

impl Chain for Polkadot {
	const ID: ChainId = *b"dotc";

	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hasher = BlakeTwo256;
	type Header = generic::Header<BlockNumber, BlakeTwo256>;

	type AccountId = AccountId;
	type Balance = Balance;
	type Index = u32; // defined as Nonce in Polkadot
	type Signature = Signature;

	const STATE_VERSION: StateVersion = StateVersion::V1;

	// TODO: Using Polkdaex params instead of polkadot
	fn max_extrinsic_size() -> u32 {
		*RuntimeBlockLength::get().max.get(DispatchClass::Normal)
	}

	// TODO: Using Polkdaex params instead of polkadot
	fn max_extrinsic_weight() -> Weight {
		RuntimeBlockWeights::get()
			.get(DispatchClass::Normal)
			.max_extrinsic
			.unwrap_or(Weight::MAX)
	}
}

impl ChainWithGrandpa for Polkadot {
	const WITH_CHAIN_GRANDPA_PALLET_NAME: &'static str = WITH_POLKADOT_GRANDPA_PALLET_NAME;
	const MAX_AUTHORITIES_COUNT: u32 = 100_000; // Taken from polkadot master branch
	const REASONABLE_HEADERS_IN_JUSTIFICATON_ANCESTRY: u32 =
		REASONABLE_HEADERS_IN_JUSTIFICATON_ANCESTRY;
	const MAX_HEADER_SIZE: u32 = MAX_HEADER_SIZE;
	const AVERAGE_HEADER_SIZE_IN_JUSTIFICATION: u32 = AVERAGE_HEADER_SIZE_IN_JUSTIFICATION;
}
