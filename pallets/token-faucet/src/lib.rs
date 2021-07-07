#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::traits::AtLeast32BitUnsigned;
use frame_support::sp_runtime::SaturatedConversion;
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch, traits::Get,
};
use frame_system::ensure_none;
use orml_traits::{MultiCurrency, MultiCurrencyExtended};

use polkadex_primitives::assets::AssetId;

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    /// Balance Type
    type Balance: Parameter
        + Member
        + AtLeast32BitUnsigned
        + Default
        + Copy
        + MaybeSerializeDeserialize;
    /// Module that handles tokens
    type Currency: MultiCurrencyExtended<
        Self::AccountId,
        CurrencyId = AssetId,
        Balance = Self::Balance,
    >;
}

// The pallet's runtime storage items.
// https://substrate.dev/docs/en/knowledgebase/runtime/storage
decl_storage! {
    trait Store for Module<T: Config> as TokenFaucetMap {
        //Total token supply
        pub TokenFaucetMap get(fn token_faucet): map hasher(blake2_128_concat) T::AccountId => T::BlockNumber;
    }
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
    {
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

        fn deposit_event() = default;

        #[weight = 10_000 + T::DbWeight::get().writes(1)]
        pub fn credit_account_with_tokens_unsigned(origin, account: T::AccountId) -> dispatch::DispatchResult {
            let _ = ensure_none(origin)?;
            TokenFaucetMap::<T>::insert(&account,<frame_system::Pallet<T>>::block_number());
            //Mint account with free tokens
            T::Currency::deposit(AssetId::POLKADEX, &account,(1000000000000000000 as u128).saturated_into())?;
            Self::deposit_event(RawEvent::AccountCredited(account));
            Ok(())
        }
    }
}

/// Number blocks created every 24 hrs
const BLOCK_THRESHOLD: u64 = (24 * 60 * 60) / 6;
/// The type to sign and send transactions.
pub const UNSIGNED_TXS_PRIORITY: u64 = 100;

impl<T: Config> frame_support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;
    fn validate_unsigned(
        _source: frame_support::unsigned::TransactionSource,
        call: &Self::Call,
    ) -> frame_support::unsigned::TransactionValidity {
        let current_block_no: T::BlockNumber = <frame_system::Pallet<T>>::block_number();

        //debu
        let valid_tx = |account: &T::AccountId| {
            let last_block_number: T::BlockNumber = <TokenFaucetMap<T>>::get(account);
            if (last_block_number == 0_u64.saturated_into())
                || (current_block_no - last_block_number >= BLOCK_THRESHOLD.saturated_into())
            {
                ValidTransaction::with_tag_prefix("token-faucet")
                    .priority(UNSIGNED_TXS_PRIORITY)
                    .and_provides([&b"request_token_faucet".to_vec()])
                    .longevity(3)
                    .propagate(true)
                    .build()
            } else {
                TransactionValidity::Err(TransactionValidityError::Invalid(
                    InvalidTransaction::ExhaustsResources,
                ))
            }
        };

        match call {
            Call::credit_account_with_tokens_unsigned(account) => valid_tx(account),
            _ => InvalidTransaction::Call.into(),
        }
    }
}
