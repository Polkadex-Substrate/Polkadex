use crate::types::GossipMessage;
use parity_scale_codec::Decode;
use parking_lot::RwLock;
use sc_network::PeerId;
use sc_network_common::protocol::role::ObservedRole;
use sc_network_gossip::{MessageIntent, ValidationResult, Validator, ValidatorContext};
use sp_runtime::traits::{Block, Hash, Header};
use sp_tracing::info;
use std::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
};
use thea_primitives::{Message, NATIVE_NETWORK};

/// Gossip engine messages topic
pub fn topic<B: Block>() -> B::Hash
where
	B: Block,
{
	<<B::Header as Header>::Hashing as Hash>::hash(b"/thea/1")
}

/// Thea gossip validator
///
/// Validate Orderbook gossip messages and limit the number of broadcast rounds.
///
/// Allows messages for 'rounds >= last concluded' to flow, everything else gets
/// rejected/expired.
///
///All messaging is handled in a single Orderbook global topic.
pub struct GossipValidator<B>
where
	B: Block,
{
	topic: B::Hash,
	pub(crate) peers: Arc<RwLock<BTreeSet<PeerId>>>,
	pub(crate) fullnodes: Arc<RwLock<BTreeSet<PeerId>>>,
	cache: Arc<RwLock<BTreeMap<Message, GossipMessage>>>,
	foreign_last_nonce: Arc<RwLock<u64>>, /* Nonce of foreign message that was last processed in
	                                       * native */
	native_last_nonce: Arc<RwLock<u64>>, /* Nonce of native message that was last processed in
	                                      * foreign */
}

impl<B> GossipValidator<B>
where
	B: Block,
{
	pub fn new(
		cache: Arc<RwLock<BTreeMap<Message, GossipMessage>>>,
		foreign_last_nonce: Arc<RwLock<u64>>,
		native_last_nonce: Arc<RwLock<u64>>,
	) -> GossipValidator<B> {
		log::debug!(target: "thea", "Creating gossip validator");
		GossipValidator {
			topic: topic::<B>(),
			peers: Arc::new(RwLock::new(BTreeSet::new())),
			fullnodes: Arc::new(RwLock::new(BTreeSet::new())),
			cache,
			foreign_last_nonce,
			native_last_nonce,
		}
	}

	pub fn validate_message(&self, message: &GossipMessage) -> bool {
		// verify the message with our message cache and foreign chain connector
		if message.payload.network == NATIVE_NETWORK {
			// Message origin is native
			self.native_last_nonce.read().lt(&message.payload.nonce)
		} else {
			// Message origin is foreign
			self.foreign_last_nonce.read().lt(&message.payload.nonce)
		}
	}

	pub fn rebroadcast_check(&self, message: &GossipMessage) -> bool {
		// We rebroadcast it as long as its in our cache, if its not in our cache,
		// then don't broadcast it, its removed from cache when the message is accepted.
		self.cache.read().contains_key(&message.payload)
	}
}

impl<B> Validator<B> for GossipValidator<B>
where
	B: Block,
{
	fn validate(
		&self,
		_context: &mut dyn ValidatorContext<B>,
		sender: &PeerId,
		_data: &[u8],
	) -> ValidationResult<B::Hash> {
		log::debug!(target: "thea", "Validator validating message from: {}", sender.to_base58());
		ValidationResult::ProcessAndKeep(self.topic)
	}
}
