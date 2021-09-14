// Copyright (C) 2020-2021 Polkadex OU
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

use sp_std::{prelude::*, result};
use cid::Cid;
pub use pallet::*;

mod mock;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        sp_runtime::traits::AtLeast32BitUnsigned,
    };
    use frame_system::pallet_prelude::*;

    use offchain_ipfs_primitives::inherents::{INHERENT_IDENTIFIER, InherentError, InherentType};

    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Balance Type
        type Balance: Parameter
        + Member
        + AtLeast32BitUnsigned
        + Default
        + Copy
        + MaybeSerializeDeserialize;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    /// Latest CID provided by enclave
    #[pallet::storage]
    #[pallet::getter(fn latest_cid)]
    pub(super) type LatestCID<T: Config> = StorageValue<_, Cid, OptionQuery>;

    /// CID approved by validators
    #[pallet::storage]
    #[pallet::getter(fn approved_cid)]
    pub(super) type ApprovedCID<T: Config> = StorageValue<_, Cid, OptionQuery>;

    /// Operational Status of exchange
    #[pallet::storage]
    #[pallet::getter(fn operational_status)]
    pub(super) type OperationalStatus<T: Config> = StorageValue<_, bool, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn user_claims)]
    pub(super) type UserClaims<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn enclave_multi_addr)]
    pub(super) type EnclaveIPFSMultiAddrs<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Vec<Vec<u8>>, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig {
        pub operational_status: bool,
    }

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            Self { operational_status: true }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            <OperationalStatus<T>>::put(true);
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Enclave inserts a new multiaddr to runtime
        #[pallet::weight(10_000)]
        pub fn add_enclave_multiaddr(origin: OriginFor<T>, multiaddr: Vec<Vec<u8>>) -> DispatchResult {
            let enclave = ensure_signed(origin)?;
            // TODO: Make sure the enclave is registered
            // TODO: Remember to delete it when it's not used anymore otherwise we are wasting runtime storage
            <EnclaveIPFSMultiAddrs<T>>::insert(enclave, multiaddr);

            Ok(())
        }

        /// Enclave inserts a new CID for approval of validators
        #[pallet::weight(10_000)]
        pub fn insert_new_cid(origin: OriginFor<T>, cid: Cid) -> DispatchResult {
            let _enclave = ensure_signed(origin)?;
            // TODO: Make sure the enclave is registered
            <LatestCID<T>>::put(cid);
            Ok(())
        }

        /// Any random validator in the current set will call this, which is also verified using inherent logic by other validators
        /// In case, CID is wrong, the given block will be rejected and the validator which inserted the inherent is penalized by staking mechanism.
        #[pallet::weight(10_000)]
        pub fn approve_cid(origin: OriginFor<T>, cid: Cid) -> DispatchResult {
            ensure_none(origin)?; // TODO: Do we need the validator to sign this inherent?
            <ApprovedCID<T>>::put(cid);
            Ok(())
        }

        /// Shutdowns the exchange and take the last approved CID as the final balance state
        #[pallet::weight(10_000)]
        pub fn emergency_stop(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?; // TODO: Make this Root of 2/3rd of general council
            <OperationalStatus<T>>::put(false);
            Ok(())
        }

        /// Users use this function to claim their balances
        #[pallet::weight(10_000)]
        pub fn claim(origin: OriginFor<T>) -> DispatchResult {
            let user = ensure_signed(origin)?;
            // TODO: User should pay the fees required for finding, setting the balance
            //		 and not only for this extrinsic
            <UserClaims<T>>::insert(user, true);
            Ok(())
        }

        /// Users use this function to claim their balances
        #[pallet::weight(10_000)]
        pub fn redeem_claim(origin: OriginFor<T>, _balance: T::Balance, user: T::AccountId) -> DispatchResult {
            // TODO: This functions should be further extended to support multiple
            // 		token claims since the user will have multiple tokens balances
            ensure_none(origin)?; // TODO: Do we need the validator to sign this inherent?
            // Note, the user deposits the balance to exchange via OCEX pallet and tokens are locked there
            // Those tokens are removed and updated via this redeem call
            // TODO: Update balance
            <UserClaims<T>>::take(user); // Claim is removed since claim is redeemed
            Ok(())
        }
    }

    #[pallet::inherent]
    impl<T: Config> ProvideInherent for Pallet<T> {
        type Call = Call<T>;
        type Error = InherentError;
        const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

        fn create_inherent(data: &InherentData) -> Option<Self::Call> {
            let inherent_data = data
                .get_data::<InherentType>(&INHERENT_IDENTIFIER)
                .expect("IPFS inherent data not correctly encoded")
                .expect("IPFS inherent data must be provided");

            Some(Call::approve_cid(inherent_data))
        }

        fn check_inherent(
            call: &Self::Call,
            data: &InherentData,
        ) -> result::Result<(), Self::Error> {
            let imported_approved_cid: Cid = match call {
                Call::approve_cid(t) => *t,
                _ => return Err(InherentError::WrongInherentCall),
            };

            let data = data
                .get_data::<InherentType>(&INHERENT_IDENTIFIER)
                .expect("IPFS inherent data not correctly encoded")
                .expect("IPFS inherent data must be provided");

            let local_approved_cid = data;

            if imported_approved_cid != local_approved_cid {
                return Err(InherentError::InvalidCID(imported_approved_cid.clone()));
            }

            Ok(())
        }

        fn is_inherent(call: &Self::Call) -> bool {
            matches!(call, Call::approve_cid(_))
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", T::Balance = "Balance")]
    pub enum Event<T: Config> {
        /// New CID from enclave
        NewCID(T::Hash),
    }

    #[pallet::error]
    pub enum Error<T> {}
}

impl<T: Config> Pallet<T> {
    /// Provides the latest cid
    pub fn get_latest_cid() -> Option<Cid> {
        <LatestCID<T>>::get()
    }
    /// True if the exchange is operational
    pub fn check_emergency_closure() -> bool {
        <OperationalStatus<T>>::get()
    }
    /// Approved CID
    pub fn get_approved_cid() -> Option<Cid> {
        <ApprovedCID<T>>::get()
    }
    /// Get all user claims
    pub fn collect_user_claims() -> Vec<T::AccountId> {
        <UserClaims<T>>::iter_keys().collect()
    }
    /// Get all Multiaddress
    pub fn collect_enclave_multiaddrs() -> Vec<(T::AccountId,Vec<Vec<u8>>)> {
        <EnclaveIPFSMultiAddrs<T>>::iter_keys().zip(<EnclaveIPFSMultiAddrs<T>>::iter_values()).collect()
    }
}
