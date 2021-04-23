/// A pair of arbitrary bytes.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Pair {
    /// The first element of the pair.
    #[prost(bytes="vec", tag="1")]
    pub fst: ::prost::alloc::vec::Vec<u8>,
    /// The second element of the pair.
    #[prost(bytes="vec", tag="2")]
    pub snd: ::prost::alloc::vec::Vec<u8>,
}
/// Enumerate all possible light client request messages.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Request {
    #[prost(oneof="request::Request", tags="1, 2, 3, 4, 5")]
    pub request: ::core::option::Option<request::Request>,
}
/// Nested message and enum types in `Request`.
pub mod request {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Request {
        #[prost(message, tag="1")]
        RemoteCallRequest(super::RemoteCallRequest),
        #[prost(message, tag="2")]
        RemoteReadRequest(super::RemoteReadRequest),
        #[prost(message, tag="3")]
        RemoteHeaderRequest(super::RemoteHeaderRequest),
        #[prost(message, tag="4")]
        RemoteReadChildRequest(super::RemoteReadChildRequest),
        #[prost(message, tag="5")]
        RemoteChangesRequest(super::RemoteChangesRequest),
    }
}
/// Enumerate all possible light client response messages.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Response {
    #[prost(oneof="response::Response", tags="1, 2, 3, 4")]
    pub response: ::core::option::Option<response::Response>,
}
/// Nested message and enum types in `Response`.
pub mod response {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Response {
        #[prost(message, tag="1")]
        RemoteCallResponse(super::RemoteCallResponse),
        #[prost(message, tag="2")]
        RemoteReadResponse(super::RemoteReadResponse),
        #[prost(message, tag="3")]
        RemoteHeaderResponse(super::RemoteHeaderResponse),
        #[prost(message, tag="4")]
        RemoteChangesResponse(super::RemoteChangesResponse),
    }
}
/// Remote call request.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RemoteCallRequest {
    /// Block at which to perform call.
    #[prost(bytes="vec", tag="2")]
    pub block: ::prost::alloc::vec::Vec<u8>,
    /// Method name.
    #[prost(string, tag="3")]
    pub method: ::prost::alloc::string::String,
    /// Call data.
    #[prost(bytes="vec", tag="4")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
/// Remote call response.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RemoteCallResponse {
    /// Execution proof.
    #[prost(bytes="vec", tag="2")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
/// Remote storage read request.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RemoteReadRequest {
    /// Block at which to perform call.
    #[prost(bytes="vec", tag="2")]
    pub block: ::prost::alloc::vec::Vec<u8>,
    /// Storage keys.
    #[prost(bytes="vec", repeated, tag="3")]
    pub keys: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
/// Remote read response.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RemoteReadResponse {
    /// Read proof.
    #[prost(bytes="vec", tag="2")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
/// Remote storage read child request.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RemoteReadChildRequest {
    /// Block at which to perform call.
    #[prost(bytes="vec", tag="2")]
    pub block: ::prost::alloc::vec::Vec<u8>,
    /// Child Storage key, this is relative
    /// to the child type storage location.
    #[prost(bytes="vec", tag="3")]
    pub storage_key: ::prost::alloc::vec::Vec<u8>,
    /// Storage keys.
    #[prost(bytes="vec", repeated, tag="6")]
    pub keys: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
/// Remote header request.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RemoteHeaderRequest {
    /// Block number to request header for.
    #[prost(bytes="vec", tag="2")]
    pub block: ::prost::alloc::vec::Vec<u8>,
}
/// Remote header response.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RemoteHeaderResponse {
    /// Header. None if proof generation has failed (e.g. header is unknown).
    ///
    /// optional
    #[prost(bytes="vec", tag="2")]
    pub header: ::prost::alloc::vec::Vec<u8>,
    /// Header proof.
    #[prost(bytes="vec", tag="3")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
//// Remote changes request.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RemoteChangesRequest {
    /// Hash of the first block of the range (including first) where changes are requested.
    #[prost(bytes="vec", tag="2")]
    pub first: ::prost::alloc::vec::Vec<u8>,
    /// Hash of the last block of the range (including last) where changes are requested.
    #[prost(bytes="vec", tag="3")]
    pub last: ::prost::alloc::vec::Vec<u8>,
    /// Hash of the first block for which the requester has the changes trie root. All other
    /// affected roots must be proved.
    #[prost(bytes="vec", tag="4")]
    pub min: ::prost::alloc::vec::Vec<u8>,
    /// Hash of the last block that we can use when querying changes.
    #[prost(bytes="vec", tag="5")]
    pub max: ::prost::alloc::vec::Vec<u8>,
    /// Storage child node key which changes are requested.
    ///
    /// optional
    #[prost(bytes="vec", tag="6")]
    pub storage_key: ::prost::alloc::vec::Vec<u8>,
    /// Storage key which changes are requested.
    #[prost(bytes="vec", tag="7")]
    pub key: ::prost::alloc::vec::Vec<u8>,
}
/// Remote changes response.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RemoteChangesResponse {
    /// Proof has been generated using block with this number as a max block. Should be
    /// less than or equal to the RemoteChangesRequest::max block number.
    #[prost(bytes="vec", tag="2")]
    pub max: ::prost::alloc::vec::Vec<u8>,
    /// Changes proof.
    #[prost(bytes="vec", repeated, tag="3")]
    pub proof: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// Changes tries roots missing on the requester' node.
    #[prost(message, repeated, tag="4")]
    pub roots: ::prost::alloc::vec::Vec<Pair>,
    /// Missing changes tries roots proof.
    #[prost(bytes="vec", tag="5")]
    pub roots_proof: ::prost::alloc::vec::Vec<u8>,
}
