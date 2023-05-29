use parity_scale_codec::{Decode, Encode};

use bls_primitives::Signature;
use thea_primitives::types::Message;

#[derive(Encode, Decode, Clone, Debug)]
pub struct GossipMessage {
	pub(crate) payload: Message,
	pub(crate) bitmap: Vec<u128>,
	pub(crate) aggregate_signature: Signature,
}
