// Payloads for Noise handshake messages.

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NoiseHandshakePayload {
    #[prost(bytes="vec", tag="1")]
    pub identity_key: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="2")]
    pub identity_sig: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="3")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
