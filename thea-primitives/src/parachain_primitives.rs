#![cfg_attr(not(feature = "std"), no_std)]

use crate::AssetIdConverter;
use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::BoundedVec;
use scale_info::TypeInfo;
use sp_runtime::traits::ConstU32;
use sp_std::{vec, vec::Vec};
use xcm::{
	latest::{Fungibility, MultiAsset, MultiLocation},
	prelude::Xcm,
};
use xcm::latest::AssetId;

#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub enum AssetType {
	Fungible,
	NonFungible
}

#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub struct ParachainAsset {
	pub location: MultiLocation,
	pub asset_type: AssetType
}

#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub struct ParachainDeposit {
	pub recipient: MultiLocation,
	pub asset_and_amount: MultiAsset,
	pub deposit_nonce: u32,
	pub transaction_hash: sp_core::H256,
	pub network_id: u8,
}

impl ParachainDeposit {
	pub fn convert_multi_asset_to_asset_id_and_amount(&self) -> Option<(u128, u128)> {
		let MultiAsset { id: _, fun } = self.asset_and_amount.clone();
		match fun {
			Fungibility::Fungible(fun) => self.get_asset_id().map(|asset| (asset, fun)),
			_ => None,
		}
	}

	pub fn get_parachain_asset(&self) -> Option<ParachainAsset> {
		let MultiAsset { id, .. } = self.asset_and_amount.clone();
		if let AssetId::Concrete(multilocation) = id {
			Some(ParachainAsset {
				location: multilocation,
				asset_type: AssetType::Fungible,
			})
		} else {
			None
		}

	}
}

impl AssetIdConverter for ParachainDeposit {
	fn get_asset_id(&self) -> Option<u128> {
		if let Some(parachain_asset) = self.get_parachain_asset() {
			if let Ok(asset_identifier) =
			BoundedVec::<u8, ConstU32<1000>>::try_from(parachain_asset.encode())
			{
				let identifier_length = asset_identifier.len();
				let mut derived_asset_id: Vec<u8> = vec![];
				derived_asset_id.push(self.network_id);
				derived_asset_id.push(identifier_length as u8);
				derived_asset_id.extend(&asset_identifier.to_vec());
				let derived_asset_id_hash =
					&sp_io::hashing::keccak_256(derived_asset_id.as_ref())[0..16];
				let mut temp = [0u8; 16];
				temp.copy_from_slice(derived_asset_id_hash);
				Some(u128::from_le_bytes(temp))
			} else {
				None
			}
		} else {
			None
		}
	}

	fn to_asset_id(&self) -> Self {
		todo!()
	}
}

#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub struct ParachainWithdraw {
	pub xcm_messages: Xcm<()>,
}
