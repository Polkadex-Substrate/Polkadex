use frame_support::pallet_prelude::TypeInfo;
use parity_scale_codec::{Decode, Encode};
use xcm::latest::Junctions::{X1, X2};
use xcm::prelude::*;

/// Extra data fields in Thea message, this can be extended
/// with new variants for future features
/// WARNING!: Don't change the order of variants
#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub enum ExtraData {
	None,
	DirectDeposit,
}

pub fn extract_data_from_multilocation(
	multi_location: &xcm::prelude::Location,
) -> Option<([u8; 32], ExtraData)> {

	match multi_location {
		// Normal deposit
		Location { parents: 0, interior: X1(acc) } => {
			let acc  = *acc.clone();
			match acc {
				[AccountId32 { id, network }] => {
					if network == Some(Polkadot) || network.is_none() {
						Some((id, ExtraData::None))
					} else {
						None
					}
				},
				_ => None
			}
		},
		// Direct deposit
		Location {
			parents: 0,
			interior: X2(acc),
		} => {
			let acc  = *acc.clone();
			match acc {
				[AccountId32 { id, network }, PalletInstance(_index)] => {
					if network == Some(Polkadot) || network.is_none() {
						Some((id, ExtraData::DirectDeposit))
					} else {
						None
					}
				}
				_ => None
			}
		},
		_ => None,
	}
}
