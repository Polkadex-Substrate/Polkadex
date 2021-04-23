#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Identify {
    /// protocolVersion determines compatibility between peers
    ///
    /// e.g. ipfs/1.0.0
    #[prost(string, optional, tag="5")]
    pub protocol_version: ::core::option::Option<::prost::alloc::string::String>,
    /// agentVersion is like a UserAgent string in browsers, or client version in bittorrent
    /// includes the client name and client.
    ///
    /// e.g. go-ipfs/0.1.0
    #[prost(string, optional, tag="6")]
    pub agent_version: ::core::option::Option<::prost::alloc::string::String>,
    /// publicKey is this node's public key (which also gives its node.ID)
    /// - may not need to be sent, as secure channel implies it has been sent.
    /// - then again, if we change / disable secure channel, may still want it.
    #[prost(bytes="vec", optional, tag="1")]
    pub public_key: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
    /// listenAddrs are the multiaddrs the sender node listens for open connections on
    #[prost(bytes="vec", repeated, tag="2")]
    pub listen_addrs: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// oservedAddr is the multiaddr of the remote endpoint that the sender node perceives
    /// this is useful information to convey to the other side, as it helps the remote endpoint
    /// determine whether its connection to the local peer goes through NAT.
    #[prost(bytes="vec", optional, tag="4")]
    pub observed_addr: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
    #[prost(string, repeated, tag="3")]
    pub protocols: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
