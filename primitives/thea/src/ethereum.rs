use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::{H160, H256, U256};
use sp_runtime::{Deserialize, Serialize};

#[derive(
	Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize,
)]
pub enum EtherumAction<AccountId> {
	/// Asset id, Amount, user address
	Deposit(H160, U256, AccountId),
	/// Asset id, Amount, user address, proxy address
	DepositToOrderbook(H160, U256, AccountId, AccountId),
	/// Swap
	Swap,
}

#[derive(
	Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize,
)]
pub struct EthereumOP<AccountId> {
	pub txn_id: H256,
	pub action: EtherumAction<AccountId>,
}
