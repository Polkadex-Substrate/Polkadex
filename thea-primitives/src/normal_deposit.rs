#![cfg_attr(not(feature = "std"), no_std)]

use crate::AssetIdConverter;
use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::BoundedVec;
use scale_info::TypeInfo;
use sp_io::hashing::keccak_256;
use sp_std::vec;

#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub struct Deposit<AccountId> {
	pub network_id: u8,
	pub recipient: AccountId,
	pub transaction_hash: sp_core::H256,
	pub asset_identifier: BoundedVec<u8, sp_runtime::traits::ConstU32<1000>>,
	pub amount: u128,
	pub deposit_nonce: u32,
}

impl<AccountId> AssetIdConverter for Deposit<AccountId> {
	fn get_asset_id(&self) -> Option<u128> {
		let mut derived_asset_id = vec![];
		derived_asset_id.push(self.network_id);
		derived_asset_id.push(self.asset_identifier.len() as u8);
		derived_asset_id.extend(self.asset_identifier.to_vec());
		let derived_asset_id_hash = &keccak_256(derived_asset_id.as_ref())[0..16];
		let mut temp = [0u8; 16];
		temp.copy_from_slice(derived_asset_id_hash);
		Some(u128::from_le_bytes(temp))
	}

	fn to_asset_id(&self) -> Self {
		todo!()
	}
}
