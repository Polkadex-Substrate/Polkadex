use log::trace;
use orderbook_primitives::types::ObMessage;
use parity_scale_codec::Decode;
use sc_network::PeerId;
use sc_network_gossip::{MessageIntent, ValidationResult, Validator, ValidatorContext};
use sp_runtime::traits::{Block, Hash, Header};

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
}

impl<B> GossipValidator<B>
where
	B: Block,
{
	pub fn new() -> GossipValidator<B> {
		GossipValidator { topic: topic::<B>() }
	}
}

impl<B> Validator<B> for GossipValidator<B>
where
	B: Block,
{
	fn validate(
		&self,
		_context: &mut dyn ValidatorContext<B>,
		_sender: &PeerId,
		mut data: &[u8],
	) -> ValidationResult<B::Hash> {
		// Decode
		if let Ok(ob_message) = ObMessage::decode(&mut data) {
			todo!()
			// Check if stid is processed then discard
			// if not processed process and keep
		}
		ValidationResult::Discard
	}

	fn message_expired<'a>(&'a self) -> Box<dyn FnMut(B::Hash, &[u8]) -> bool + 'a> {
		Box::new(move |_topic, mut data| {
			// Decode
			let msg = match ObMessage::decode(&mut data) {
				Ok(vote) => vote,
				Err(_) => return true,
			};
			// If old stid then expire
			todo!();
		})
	}

	fn message_allowed<'a>(
		&'a self,
	) -> Box<dyn FnMut(&PeerId, MessageIntent, &B::Hash, &[u8]) -> bool + 'a> {
		Box::new(move |_who, intent, _topic, mut data| {
			let msg = match ObMessage::decode(&mut data) {
				Ok(vote) => vote,
				Err(_) => return false,
			};

			todo!()
			// Logic for rebroadcasting.
		})
	}
}
