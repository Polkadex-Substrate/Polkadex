use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::{H160, RuntimeDebug};

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, PartialOrd, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AssetId {
    POLKADEX,
    // DOT, TODO: Enabled in Parachain upgrade
    CHAINSAFE(H160),
    SNOWFORK(H160),
    // PARACHAIN(para_id,network,palletInstance,assetID)
}