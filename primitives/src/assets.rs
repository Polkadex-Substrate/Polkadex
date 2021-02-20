use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::{H160, RuntimeDebug};

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, PartialOrd, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AssetId {
    POLKADEX,
    // DOT, TODO: Enabled in Parachain upgrade
    ChainsafeErc20(H160),
    ChainsafeErc721(H160),
    // PARACHAIN(para_id,network,palletInstance,assetID) TODO: Enabled in parachain upgrade
}