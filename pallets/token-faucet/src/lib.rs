#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame

use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch, traits::Get, debug,
                    traits::{Currency, ExistenceRequirement::AllowDeath, Imbalance, OnUnbalanced},
};
use frame_system::{ensure_none, ensure_signed};
use frame_support::pallet_prelude::*;
use frame_system::offchain::SubmitTransaction;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    type Currency: Currency<Self::AccountId>;
}

// The pallet's runtime storage items.
// https://substrate.dev/docs/en/knowledgebase/runtime/storage
decl_storage! {
	trait Store for Module<T: Config> as TokenFaucetMap {
        //Total token supply
		pub TokenFaucetMap get(fn token_faucet): map hasher(blake2_128_concat) T::AccountId => u64;
	}
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Config>::AccountId {
		AccountCredited(AccountId),
	}
);

// Errors inform users that something went wrong.
decl_error! {
	pub enum Error for Module<T: Config> {
		AccountAlreadyCredited,
	}
}

// Dispatchable functions allows users to interact with the pallet and invoke state changes.
// These functions materialize as "extrinsics", which are often compared to transactions.
// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		// Errors must be initialized if they are used by the pallet.
		type Error = Error<T>;


		#[weight = 10_000 + T::DbWeight::get().writes(1)]
		pub fn credit_account_with_tokens_unsigned(origin, block_number: u64) -> dispatch::DispatchResult {
            let account = ensure_signed(origin)?;
            TokenFaucetMap::<T>::insert(&account,block_number);
            //Mint account with free tokens
            T::Currency::deposit_creating(&account, 100000).map_err(|_| DispatchError::Other("Minting failed"))?;;
			Self::deposit_event(RawEvent::AccountCredited(origin));
			Ok(())
		}
	}
}


impl<T: Config> Module<T> {
    fn offchain_unsigned_tx(block_number: T::BlockNumber) -> Result<(), Error<T>> {
        let block_number: u64 = block_number.try_into().unwrap_or(0);
        let call = Call::credit_account_with_tokens_unsigned(block_number);
        SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).map_err(|_| {
            debug::error!("Failed in offchain_unsigned_tx");
            <Error<T>>::OffchainUnsignedTxError
        })
    }
}
/// Number blocks created every 24 hrs
const BLOCK_THRESHOLD : u64 = ((24 * 60 * 60) / 6);

impl<T: Config> frame_support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;
    fn validate_unsigned(_source: frame_support::unsigned::TransactionSource, call: &Self::Call)
                         -> frame_support::unsigned::TransactionValidity {
        let current_block_no: u64 = <frame_system::Pallet<T>>::block_number().try_into().unwrap_or(0);

        let valid_tx = |block_number: u64| {
            current_block_no - block_number >= BLOCK_THRESHOLD
        };

        match call {
            Call::credit_account_with_tokens_unsigned(block_number) => {
                valid_tx(*block_number)
            }
            _ => InvalidTransaction::Call.into(),
        }
    }
}