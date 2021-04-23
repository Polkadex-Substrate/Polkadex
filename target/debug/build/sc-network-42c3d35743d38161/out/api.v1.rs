/// Request block data from a peer.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockRequest {
    /// Bits of block data to request.
    #[prost(uint32, tag="1")]
    pub fields: u32,
    /// End at this block. An implementation defined maximum is used when unspecified.
    ///
    /// optional
    #[prost(bytes="vec", tag="4")]
    pub to_block: ::prost::alloc::vec::Vec<u8>,
    /// Sequence direction.
    #[prost(enumeration="Direction", tag="5")]
    pub direction: i32,
    /// Maximum number of blocks to return. An implementation defined maximum is used when unspecified.
    ///
    /// optional
    #[prost(uint32, tag="6")]
    pub max_blocks: u32,
    /// Start from this block.
    #[prost(oneof="block_request::FromBlock", tags="2, 3")]
    pub from_block: ::core::option::Option<block_request::FromBlock>,
}
/// Nested message and enum types in `BlockRequest`.
pub mod block_request {
    /// Start from this block.
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum FromBlock {
        /// Start with given hash.
        #[prost(bytes, tag="2")]
        Hash(::prost::alloc::vec::Vec<u8>),
        /// Start with given block number.
        #[prost(bytes, tag="3")]
        Number(::prost::alloc::vec::Vec<u8>),
    }
}
/// Response to `BlockRequest`
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockResponse {
    /// Block data for the requested sequence.
    #[prost(message, repeated, tag="1")]
    pub blocks: ::prost::alloc::vec::Vec<BlockData>,
}
/// Block data sent in the response.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockData {
    /// Block header hash.
    #[prost(bytes="vec", tag="1")]
    pub hash: ::prost::alloc::vec::Vec<u8>,
    /// Block header if requested.
    ///
    /// optional
    #[prost(bytes="vec", tag="2")]
    pub header: ::prost::alloc::vec::Vec<u8>,
    /// Block body if requested.
    ///
    /// optional
    #[prost(bytes="vec", repeated, tag="3")]
    pub body: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// Block receipt if requested.
    ///
    /// optional
    #[prost(bytes="vec", tag="4")]
    pub receipt: ::prost::alloc::vec::Vec<u8>,
    /// Block message queue if requested.
    ///
    /// optional
    #[prost(bytes="vec", tag="5")]
    pub message_queue: ::prost::alloc::vec::Vec<u8>,
    /// Justification if requested.
    ///
    /// optional
    #[prost(bytes="vec", tag="6")]
    pub justification: ::prost::alloc::vec::Vec<u8>,
    /// True if justification should be treated as present but empty.
    /// This hack is unfortunately necessary because shortcomings in the protobuf format otherwise
    /// doesn't make in possible to differentiate between a lack of justification and an empty
    /// justification.
    ///
    /// optional, false if absent
    #[prost(bool, tag="7")]
    pub is_empty_justification: bool,
}
/// Block enumeration direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Direction {
    /// Enumerate in ascending order (from child to parent).
    Ascending = 0,
    /// Enumerate in descending order (from parent to canonical child).
    Descending = 1,
}
