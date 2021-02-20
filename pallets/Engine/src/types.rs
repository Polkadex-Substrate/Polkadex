use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, PartialOrd, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum OrderType {
    BidLimit,
    BidMarket,
    AskLimit,
    AskMarket,
}


#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, PartialOrd, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Order<Balance, AccountId, Signature> {
    pub price: Balance,
    pub quantity: Balance,
    pub order_type: OrderType,
    pub trader: AccountId,
    pub nonce: u64,
    pub signature: Signature,
}

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, PartialOrd, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AccountData<Balance> {
    nonce: u64, // TODO: Store nonce in a better data structure
    free_balance: Balance,
    reserved_balance: Balance // TODO: Implement a data structure to store balances of all assets
}

impl<Balance: Default> Default for AccountData<Balance> {
    fn default() -> Self {
        AccountData{
            nonce: 0,
            free_balance: Default::default(),
            reserved_balance: Default::default()
        }
    }
}
