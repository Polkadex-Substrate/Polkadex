#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CircuitRelay {
    /// Type of the message
    #[prost(enumeration="circuit_relay::Type", optional, tag="1")]
    pub r#type: ::core::option::Option<i32>,
    /// srcPeer and dstPeer are used when Type is HOP or STOP
    #[prost(message, optional, tag="2")]
    pub src_peer: ::core::option::Option<circuit_relay::Peer>,
    #[prost(message, optional, tag="3")]
    pub dst_peer: ::core::option::Option<circuit_relay::Peer>,
    /// Status code, used when Type is STATUS
    #[prost(enumeration="circuit_relay::Status", optional, tag="4")]
    pub code: ::core::option::Option<i32>,
}
/// Nested message and enum types in `CircuitRelay`.
pub mod circuit_relay {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Peer {
        /// peer id
        #[prost(bytes="vec", required, tag="1")]
        pub id: ::prost::alloc::vec::Vec<u8>,
        /// peer's known addresses
        #[prost(bytes="vec", repeated, tag="2")]
        pub addrs: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Status {
        Success = 100,
        HopSrcAddrTooLong = 220,
        HopDstAddrTooLong = 221,
        HopSrcMultiaddrInvalid = 250,
        HopDstMultiaddrInvalid = 251,
        HopNoConnToDst = 260,
        HopCantDialDst = 261,
        HopCantOpenDstStream = 262,
        HopCantSpeakRelay = 270,
        HopCantRelayToSelf = 280,
        StopSrcAddrTooLong = 320,
        StopDstAddrTooLong = 321,
        StopSrcMultiaddrInvalid = 350,
        StopDstMultiaddrInvalid = 351,
        StopRelayRefused = 390,
        MalformedMessage = 400,
    }
    /// RPC identifier, either HOP, STOP or STATUS
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Type {
        Hop = 1,
        Stop = 2,
        Status = 3,
        /// is peer a relay?
        CanHop = 4,
    }
}
