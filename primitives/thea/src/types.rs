use crate::Network;
use parity_scale_codec::{Encode,Decode};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq)]
pub struct Message {
    pub block_no: u64,
    pub nonce: u64,
    pub data: Vec<u8>,
    pub network: Network // going out to this network
}
