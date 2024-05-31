use frame_support::pallet_prelude::TypeInfo;
use parity_scale_codec::{Decode, Encode};
use xcm::v3::MultiLocation;
use xcm::v3::prelude::{AccountId32, PalletInstance};
use xcm::v3::NetworkId;
use xcm::v3::prelude::{X1, X2};

/// Extra data fields in Thea message, this can be extended
/// with new variants for future features
/// WARNING!: Don't change the order of variants
#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub enum ExtraData {
	None,
	DirectDeposit,
}

pub fn extract_data_from_multilocation(
	multi_location: xcm::v3::MultiLocation,
) -> Option<([u8; 32], ExtraData)> {
	match multi_location {
		// Normal deposit
		MultiLocation { parents: 0, interior: X1(AccountId32 { id, network }) } => {
			if network == Some(NetworkId::Polkadot) || network.is_none() {
				Some((id, ExtraData::None))
			} else {
				None
			}
		},
		// Direct deposit
		MultiLocation {
			parents: 0,
			interior: X2(AccountId32 { id, network }, PalletInstance(_index)),
		} => {
			if network == Some(NetworkId::Polkadot) || network.is_none() {
				Some((id, ExtraData::DirectDeposit))
			} else {
				None
			}
		},
		_ => None,
	}
}
