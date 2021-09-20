use crate::{errors::Error, models::TokenAddress};
use ink_env::Environment;
use ink_lang as ink;

/// Define the operations to interact with the substrate runtime
#[ink::chain_extension]
pub trait CurrencyExtension {
    type ErrorCode = Error;

    #[ink(extension = 0, returns_result = false)]
    fn deposit(
        token_address: TokenAddress,
        from: <ink_env::DefaultEnvironment as Environment>::AccountId,
        amount: <ink_env::DefaultEnvironment as Environment>::Balance,
    ) -> ();
    #[ink(extension = 1, returns_result = false)]
    fn withdraw(
        token_address: TokenAddress,
        to: <ink_env::DefaultEnvironment as Environment>::AccountId,
        amount: <ink_env::DefaultEnvironment as Environment>::Balance,
    ) -> ();
}

impl ink_env::chain_extension::FromStatusCode for Error {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            0 => Ok(()),
            1 => Err(Self::TransferFailed),
            _ => panic!("encountered unknown status code"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct CustomEnvironment;

impl Environment for CustomEnvironment {
    const MAX_EVENT_TOPICS: usize = <ink_env::DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;

    type AccountId = <ink_env::DefaultEnvironment as Environment>::AccountId;
    type Balance = <ink_env::DefaultEnvironment as Environment>::Balance;
    type Hash = <ink_env::DefaultEnvironment as Environment>::Hash;
    type BlockNumber = <ink_env::DefaultEnvironment as Environment>::BlockNumber;
    type Timestamp = <ink_env::DefaultEnvironment as Environment>::Timestamp;

    type ChainExtension = CurrencyExtension;
}
