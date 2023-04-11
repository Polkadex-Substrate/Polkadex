use parity_scale_codec::{Decode, Encode};
use thea_primitives::{types::Message, Network};

#[derive(Encode, Decode)]
pub struct GossipMessage {
	payload: Message,
	bitmap: Vec<u128>,
}
