use codec::{ Decode, Encode};
use sp_inherents:: {InherentIdentifier, IsFatalError};
use cid::Cid;

/// Offchain IPFS Inherents
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"ipfsofch";

pub type InherentType = Cid;

/// Errors that can occur while checking the Offchain IPFS inherent.
#[derive(Encode, sp_runtime::RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
pub enum InherentError {
    /// This is a fatal-error and will stop block import.
    #[cfg_attr(feature = "std", error("The inserted shared public key is invalid."))]
    InvalidCID(InherentType),
    /// This is a fatal-error and will stop block import.
    #[cfg_attr(feature = "std", error("Wrong Inherent Call in Block"))]
    WrongInherentCall,
}

impl IsFatalError for InherentError {
    fn is_fatal_error(&self) -> bool {
        match self {
            InherentError::InvalidCID(_) => true,
            InherentError::WrongInherentCall => true,
        }
    }
}

impl InherentError {
    /// Try to create an instance ouf of the given identifier and data.
    #[cfg(feature = "std")]
    pub fn try_from(id: &InherentIdentifier, data: &[u8]) -> Option<Self> {
        if id == &INHERENT_IDENTIFIER {
            <InherentError as codec::Decode>::decode(&mut &data[..]).ok()
        } else {
            None
        }
    }
}