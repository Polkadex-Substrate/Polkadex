// This file is part of Polkadex.

// Copyright (C) 2020-2022 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        dispatch::fmt::Debug,
        log,
        pallet_prelude::*,
        traits::{
            tokens::fungibles::{Create, Inspect, Mutate},
            Currency, ExistenceRequirement, ReservableCurrency,
        },
        PalletId,
    };
    use frame_system::pallet_prelude::*;
    use sp_core::{H160, U256};
    use sp_io::hashing::keccak_256;
    use sp_runtime::{
        traits::{One, Saturating, UniqueSaturatedInto},
        BoundedBTreeSet, SaturatedConversion,
    };
    use sp_std::{vec, vec::Vec};

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    /// Configure the pallet by specifying the parameters and types on which it depends.
    pub trait Config: frame_system::Config + asset_handler::pallet::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Balances Pallet
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
        /// Asset Create/ Update Origin
        type PoolCreateUpdateOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;
        /// Thea PalletId
        #[pallet::constant]
        type SwapPalletId: Get<PalletId>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // Pallets use events to inform users when important changes are made.
    // https://substrate.dev/docs/en/knowledgebase/runtime/events
    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {}

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {}

    // Hooks for Swap Pallet are defined here
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    // Extrinsic for Swap Pallet are defined here
    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    // Helper Functions for Swap Pallet
    impl<T: Config> Pallet<T> {}
}