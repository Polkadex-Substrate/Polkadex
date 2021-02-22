#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, Parameter};
use frame_support::sp_std::fmt::Debug;
use frame_support::traits::Get;
use frame_system::ensure_signed;
use sp_core::Hasher;
use sp_runtime::traits::{AtLeast32BitUnsigned, IdentifyAccount, MaybeSerializeDeserialize, Member, Verify};

use types::{AccountData, Order, OrderType::AskLimit, OrderType::AskMarket, OrderType::BidLimit, OrderType::BidMarket};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;
mod types;

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    /// Balance Type
    type Balance: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + Debug + MaybeSerializeDeserialize;
    /// Public Key of the trader
    type Public: IdentifyAccount<AccountId=Self::AccountId>;
    /// Signature provided by the trade
    type Signature: Verify<Signer=Self::Public> + Member + Decode + Encode;
}

decl_storage! {
	trait Store for Module<T: Config> as Engine {
	    Providers get(fn get_providers): map hasher(blake2_128_concat) T::AccountId => Option<u32>;
	    Traders get(fn get_traders): map hasher(blake2_128_concat) T::AccountId => AccountData<T::Hash,T::Balance>;
	}
}


decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Config>::AccountId {
		/// parameters. [something, who]
		SomethingStored(u32, AccountId),
	}
);

// Errors inform users that something went wrong.
decl_error! {
	pub enum Error for Module<T: Config> {
		/// The caller is not registered on the blockchain
		CallerNotARegisteredProvider,
		/// Trader Signature mismatch
		TraderSignatureMismatch,
		/// Outdated Trade
		NonceAlreadyUsed,
		/// OrderType Given For Maker and Taker is invalid
		InvalidOrderTypeCombination,
	}
}


decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		// Errors must be initialized if they are used by the pallet.
		type Error = Error<T>;

		// Events must be initialized if they are used by the pallet.
		fn deposit_event() = default;

		#[weight = 0]
		pub fn settle_trade(origin, maker: Order<T::Balance, T::AccountId, T::Signature>, taker: Order<T::Balance, T::AccountId, T::Signature>) -> dispatch::DispatchResult {
			let cloud_provider = ensure_signed(origin)?;
			Self::settle(cloud_provider, maker, taker)?;
			// Return a successful DispatchResult
			Ok(())
		}
	}
}

impl<T: Config> Module<T> {
    fn settle(provider: T::AccountId, maker: Order<T::Balance, T::AccountId, T::Hash, T::Signature>, taker: Order<T::Balance, T::AccountId, T::Hash, T::Signature>) -> Result<(), Error<T>> {
        // Checks if the caller is a registered member of callers
        if <Providers<T>>::contains_key(provider) {
            // Checks if the signatures are valid for maker and taker
            if Self::verify_signatures(&maker, &taker) {
                // Verify nonce
                let maker_account: AccountData<T::Hash, T::Balance> = <Traders<T>>::get(&maker.trader);
                let taker_account: AccountData<T::Hash, T::Balance> = <Traders<T>>::get(&taker.trader);
                if Self::verify_nonces(&maker_account, &maker, &taker_account, &taker) {
                    Self::execute(&maker_account, &maker, &taker_account, &taker)?;
                    Ok(())
                } else {
                    Err(Error::<T>::NonceAlreadyUsed)
                }
            } else {
                Err(Error::<T>::TraderSignatureMismatch)
            }
        } else {
            Err(Error::<T>::CallerNotARegisteredProvider)
        }
    }

    fn verify_signatures(maker: &Order<T::Balance, T::AccountId, T::Hash, T::Signature>, taker: &Order<T::Balance, T::AccountId, T::Hash, T::Signature>) -> bool {
        let maker_msg = (maker.price, maker.quantity, maker.order_type, maker.nonce).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let taker_msg = (taker.price, taker.quantity, taker.order_type, taker.nonce).using_encoded(<T as frame_system::Config>::Hashing::hash);
        maker.signature.verify(&(maker_msg.encode()[..]), &maker.trader) && taker.signature.verify(&(taker_msg.encode()[..]), &taker.trader)
    }

    /// When verifying nonce take into account,
    /// 1) Partial Orders ( These come to blockchain more than once)
    /// 2) Cancelled Orders (These won't come to blockchain)
    /// 3) Storage Access ( Storage shouldn't increase too much)
    /// 4) Easy to Verify
    /// The first principle is to prevent replay attacks.
    fn verify_nonces(maker_account: &AccountData<T::Hash, T::Balance>, maker: &Order<T::Balance, T::AccountId, T::Hash, T::Signature>,
                     taker_account: &AccountData<T::Hash, T::Balance>, taker: &Order<T::Balance, T::AccountId, T::Hash, T::Signature>) -> bool {
        // FIXME: Implement an efficient nonce verification

        true
    }

    /// TODO: Transfer the funds between maker & taker
    fn execute(mut maker_account: &AccountData<T::Hash, T::Balance>, maker: &Order<T::Balance, T::AccountId, T::Hash, T::Signature>,
               mut taker_account: &AccountData<T::Hash, T::Balance>, taker: &Order<T::Balance, T::AccountId, T::Hash, T::Signature>) -> Result<(), Error<T>> {
        match (maker.order_type, taker.order_type) {
            (BidLimit, AskLimit) => {

            }
            (BidLimit, AskMarket) => {}
            (AskLimit, BidLimit) => {}
            (AskLimit, BidMarket) => {}
            _ => {
                Err(Error::<T>::InvalidOrderTypeCombination)
            }
        }
        Ok(())
    }
}