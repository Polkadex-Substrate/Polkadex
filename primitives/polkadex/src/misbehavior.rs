use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

///
#[derive(Debug, Decode, Encode, TypeInfo, Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum TheaMisbehavior {
	// Bad BLS signature
	BlsProtocolMisconduct,
	// Bad informatino
	FaultyDataProvided,
	// Offline during keygen stage
	UnattendedKeygen,
	// Offline during offline stage
	UnattendedOffline,
	// Offline during signing stage
	UnattendedSigning,
}
