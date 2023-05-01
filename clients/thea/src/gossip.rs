use crate::types::GossipMessage;
use log::trace;
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
	<<B::Header as Header>::Hashing as Hash>::hash(b"thea")
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
	_topic: B::Hash,
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
		GossipValidator {
			_topic: topic::<B>(),
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
	fn new_peer(&self, _context: &mut dyn ValidatorContext<B>, who: &PeerId, role: ObservedRole) {
		info!(target:"thea", "New peer connected: id: {:?} role: {:?}",who,role);
		match role {
			ObservedRole::Authority => {
				self.peers.write().insert(*who);
			},
			ObservedRole::Full => {
				self.fullnodes.write().insert(*who);
			},
			_ => {},
		};
	}

	fn peer_disconnected(&self, _context: &mut dyn ValidatorContext<B>, who: &PeerId) {
		info!(target:"thea", "New peer connected: id: {:?}",who);
		self.peers.write().remove(who);
		self.fullnodes.write().remove(who);
	}

	fn validate(
		&self,
		_context: &mut dyn ValidatorContext<B>,
		_sender: &PeerId,
		mut data: &[u8],
	) -> ValidationResult<B::Hash> {
		// Decode
		if let Ok(thea_gossip_msg) = GossipMessage::decode(&mut data) {
			// Check if we processed this message
			if self.validate_message(&thea_gossip_msg) {
				trace!(target:"thea-gossip", "Validation successfully for message: {thea_gossip_msg:?}");
				return ValidationResult::ProcessAndKeep(topic::<B>())
			}
		}
		ValidationResult::Discard
	}

	fn message_expired<'a>(&'a self) -> Box<dyn FnMut(B::Hash, &[u8]) -> bool + 'a> {
		Box::new(move |_topic, mut data| {
			// Decode
			let msg = match GossipMessage::decode(&mut data) {
				Ok(msg) => msg,
				Err(_) => return true,
			};
			// If old stid then expire
			let result = !self.validate_message(&msg);
			trace!(target:"thea-gossip", "message: {msg:?} is expired: {result:?}");
			result
		})
	}

	fn message_allowed<'a>(
		&'a self,
	) -> Box<dyn FnMut(&PeerId, MessageIntent, &B::Hash, &[u8]) -> bool + 'a> {
		Box::new(move |_who, _intent, _topic, mut data| {
			// Decode
			let msg = match GossipMessage::decode(&mut data) {
				Ok(msg) => msg,
				Err(_) => return false,
			};
			// Logic for rebroadcasting.
			let result = self.rebroadcast_check(&msg);
			trace!(target:"thea-gossip", "message: {msg:?} can be rebroadcasted: {result:?}");
			result
		})
	}
}
