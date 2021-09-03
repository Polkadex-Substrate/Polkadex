// This file is part of Polkadex.

// Copyright (C) 2020-2021 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::traits::AtLeast32BitUnsigned;
use frame_support::sp_runtime::SaturatedConversion;
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame
use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use orml_traits::MultiCurrencyExtended;
use polkadex_primitives::assets::AssetId;
use sp_core::{H160, U256};
use sp_runtime::traits::StaticLookup;

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

    type CallOrigin: EnsureOrigin<Self::Origin, Success = H160>;
}

decl_storage! {
    trait Store for Module<T: Config> as NativePDEXMigration {
        /// Address of ERC20 to Native PDEX migration contract
        Address get(fn address) config(): H160;
    }
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        Balance = <T as Config>::Balance,
    {
        NativePDEXMinted(H160, H160, AccountId, U256, Balance),
    }
);

// Errors inform users that something went wrong.
decl_error! {
    pub enum Error for Module<T: Config> {
        /// The submitted payload could not be decoded.
        InvalidPayload,
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

        #[weight = 10000]
        pub fn mint(origin, token: H160, sender: H160, recipient: <T::Lookup as StaticLookup>::Source, amount: U256) -> DispatchResult {
            let who = T::CallOrigin::ensure_origin(origin)?;
            if who != Address::get() {
                return Err(DispatchError::BadOrigin.into());
            }

            let recipient = T::Lookup::lookup(recipient)?;
            // TODO: Convert U256 amount to T::Balance amount
            // T::Currency::deposit(AssetId::POLKADEX, &recipient, amount)?;
            Self::deposit_event(RawEvent::NativePDEXMinted(token, sender, recipient, amount,0_u128.saturated_into()));

            Ok(())
        }
    }
}
