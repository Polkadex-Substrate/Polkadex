use log::info;
use orderbook_primitives::{
	types::{GossipMessage, ObMessage},
	SnapshotSummary,
};
use parity_scale_codec::Decode;
use parking_lot::RwLock;
use polkadex_primitives::AccountId;
use sc_network::PeerId;
use sc_network_common::protocol::role::ObservedRole;
use sc_network_gossip::{MessageIntent, ValidationResult, Validator, ValidatorContext};
use sp_runtime::traits::{Block, Hash, Header};
use std::{collections::BTreeSet, sync::Arc};

/// Gossip engine messages topic
pub fn topic<B: Block>() -> B::Hash
where
	B: Block,
{
	<<B::Header as Header>::Hashing as Hash>::hash(b"orderbook")
}

/// Orderbook gossip validator
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
	latest_stid: Arc<RwLock<u64>>,
	pub(crate) peers: Arc<RwLock<BTreeSet<PeerId>>>,
	pub(crate) fullnodes: Arc<RwLock<BTreeSet<PeerId>>>,
}

impl<B> GossipValidator<B>
where
	B: Block,
{
	pub fn new(latest_stid: Arc<RwLock<u64>>) -> GossipValidator<B> {
		GossipValidator {
			_topic: topic::<B>(),
			latest_stid,
			peers: Arc::new(RwLock::new(BTreeSet::new())),
			fullnodes: Arc::new(RwLock::new(BTreeSet::new())),
		}
	}

	pub fn validate_message(&self, message: &GossipMessage) -> bool {
		info!(target:"orderbook","Validating message with stid: {:?}",message);
		match message {
			GossipMessage::ObMessage(msg) => {
				let last_stid = *self.latest_stid.read();
				msg.stid > last_stid
			},
			_ => true,
		}
	}

	pub fn rebroadcast_check(&self, message: &GossipMessage) -> bool {
		info!(target:"orderbook","Rebroadcast check for : {:?}",message);
		match message {
			GossipMessage::ObMessage(msg) => {
				let last_stid = *self.latest_stid.read();
				msg.stid >= last_stid
			},
			_ => true,
		}
	}
}

impl<B> Validator<B> for GossipValidator<B>
where
	B: Block,
{
	fn new_peer(&self, _context: &mut dyn ValidatorContext<B>, who: &PeerId, role: ObservedRole) {
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
		if let Ok(ob_message) = GossipMessage::decode(&mut data) {
			// Check if we processed this message
			if self.validate_message(&ob_message) {
				return ValidationResult::ProcessAndKeep(topic::<B>())
			}
			// TODO: When should be stop broadcasting this message
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
			!self.validate_message(&msg)
		})
	}

	fn message_allowed<'a>(
		&'a self,
	) -> Box<dyn FnMut(&PeerId, MessageIntent, &B::Hash, &[u8]) -> bool + 'a> {
		Box::new(move |_who, _intent, _topic, mut data| {
			// Decode
			let msg = match GossipMessage::decode(&mut data) {
				Ok(vote) => vote,
				Err(_) => return false,
			};
			// Logic for rebroadcasting.
			self.rebroadcast_check(&msg)
		})
	}
}
