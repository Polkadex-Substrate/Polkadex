use orderbook_primitives::{types::GossipMessage, SnapshotSummary};
use parity_scale_codec::Decode;
use parking_lot::RwLock;
use sc_network::PeerId;
use sc_network_common::protocol::event::ObservedRole;
use sc_network_gossip::{MessageIntent, ValidationResult, Validator, ValidatorContext};
use sp_runtime::traits::{Block, Hash, Header};
use std::sync::Arc;

/// Gossip engine messages topic
pub fn topic<B: Block>() -> B::Hash
where
	B: Block,
{
	<<B::Header as Header>::Hashing as Hash>::hash(b"orderbook")
}

/// A type that represents hash of the message.
pub type MessageHash = [u8; 8];

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
	topic: B::Hash,
	last_snapshot: Arc<RwLock<SnapshotSummary>>,
	pub(crate) peers: Vec<PeerId>,
}

impl<B> GossipValidator<B>
where
	B: Block,
{
	pub fn new(last_snapshot: Arc<RwLock<SnapshotSummary>>) -> GossipValidator<B> {
		GossipValidator { topic: topic::<B>(), last_snapshot, peers: vec![] }
	}

	pub fn validate_message(&self, _message: &GossipMessage) -> bool {
		todo!()
		// let last_snapshot = self.last_snapshot.read();
		// message.stid >= *last_snapshot.state_change_id
	}

	pub fn rebroadcast_check(&self, _message: &GossipMessage) -> bool {
		// TODO: When should we rebroadcast a message
		true
	}
}

impl<B> Validator<B> for GossipValidator<B>
where
	B: Block,
{
	fn new_peer(&self, _context: &mut dyn ValidatorContext<B>, _who: &PeerId, _role: ObservedRole) {
		todo!()
	}
	fn peer_disconnected(&self, _context: &mut dyn ValidatorContext<B>, _who: &PeerId) {
		todo!()
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