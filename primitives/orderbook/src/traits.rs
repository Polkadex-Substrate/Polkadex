use crate::SnapshotSummary;
use parity_scale_codec::{Encode, Decode};
use scale_info::TypeInfo;
use polkadex_primitives::Signature;

sp_api::decl_runtime_apis! {
    pub trait OrderbookApi {
        fn submit_snapshot(snapshot: SnapshotSummary, signature: Signature, rng: u64) -> Result<(),SigningError>;
    }
}

#[derive(Decode, Encode, TypeInfo, PartialEq, Debug)]
pub enum SigningError {
    OffchainUnsignedTxError,
}

impl core::fmt::Display for SigningError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "OffchainUnsignedTxError")
    }
}