use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use polkadex_primitives::AccountId;
use scale_info::TypeInfo;
use sp_core;
use sp_std::vec::Vec;
pub type Network = u8;
pub type SessionIndex = u32;

#[derive(Encode, Decode, Clone, Debug, MaxEncodedLen, TypeInfo, Copy)]
pub struct ApprovedDeposit {
	pub asset_id: u128,
	pub amount: u128,
	pub tx_hash: sp_core::H256,
}

#[derive(Encode, Decode, Clone, Debug, TypeInfo)]
pub struct ApprovedWithdraw {
	pub asset_id: u128,
	pub amount: u128,
	pub network: u8,
	pub beneficiary: Vec<u8>,
}

#[derive(Encode, Decode, Clone, MaxEncodedLen, TypeInfo, PartialEq, Debug)]
pub struct Payload<AccountId> {
	pub network_id: u8,
	pub who: AccountId,
	pub tx_hash: sp_core::H256,
	pub asset_id: u128,
	pub amount: u128,
	pub deposit_nonce: u32,
}

impl Payload<AccountId> {
	pub fn new(network_id: u8, who: AccountId, amount: u128, deposit_nonce: u32) -> Self {
		Payload {
			network_id,
			who,
			tx_hash: sp_core::H256::zero(),
			asset_id: 1,
			amount,
			deposit_nonce,
		}
	}
}
