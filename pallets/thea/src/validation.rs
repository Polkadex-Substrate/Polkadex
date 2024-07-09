// This file is part of Polkadex.
//
// Copyright (c) 2022-2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::{
	pallet::{
		ActiveNetworks, Authorities, OutgoingMessages, SignedOutgoingMessages, SignedOutgoingNonce,
		ValidatorSetId,
	},
	Call, Config, Pallet, THEA,
};
use frame_system::{offchain::SubmitTransaction, pallet_prelude::BlockNumberFor};
use parity_scale_codec::Encode;
use sp_application_crypto::RuntimeAppPublic;
use sp_std::vec::Vec;
use thea_primitives::Network;
use thea_primitives::types::PayloadType;

impl<T: Config> Pallet<T> {
	/// Starts the offchain worker instance that checks for finalized next incoming messages
	/// for both solochain and parachain, signs it and submits to aggregator
	pub fn run_thea_validation(_blk: BlockNumberFor<T>) -> Result<(), &'static str> {
		if !sp_io::offchain::is_validator() {
			return Ok(());
		}

		let mut id = <ValidatorSetId<T>>::get();
		let id_prev = id.saturating_sub(1);

		let authorities = <Authorities<T>>::get(id).to_vec();
		let prev_authorities =  <Authorities<T>>::get(id_prev).to_vec();

		let local_keys = T::TheaId::all();

		let mut available_keys = authorities
			.iter()
			.enumerate()
			.filter_map(move |(auth_index, authority)| {
				local_keys
					.binary_search(authority)
					.ok()
					.map(|location| (auth_index, local_keys[location].clone()))
			})
			.collect::<Vec<(usize, T::TheaId)>>();
		available_keys.sort();


		let (mut auth_index, signer) = available_keys.first().ok_or("No active keys available")?;
		log::info!(target: "thea", "Auth Index {:?} signer {:?}", auth_index, signer.clone());

		let local_keys = T::TheaId::all();
		// Fetching the available keys from previous set
		let mut prev_available_keys = prev_authorities
			.iter()
			.enumerate()
			.filter_map(move |(auth_index, authority)| {
				local_keys
					.binary_search(authority)
					.ok()
					.map(|location| (auth_index, local_keys[location].clone()))
			})
			.collect::<Vec<(usize, T::TheaId)>>();
		prev_available_keys.sort();

		let active_networks = <ActiveNetworks<T>>::get();
		log::info!(target:"thea","List of active networks: {:?}",active_networks);

		let mut signed_messages: Vec<(Network, u64, T::Signature)> = Vec::new();
		// 2. Check for new nonce to process for all networks
		for network in active_networks {
			// Sign message for each network
			let next_outgoing_nonce = <SignedOutgoingNonce<T>>::get(network).saturating_add(1);
			log::info!(target:"thea","Next outgoing nonce for network {:?} is: {:?} ",network, next_outgoing_nonce);
			// Check if we already signed it, then continue
			match <SignedOutgoingMessages<T>>::get(network, next_outgoing_nonce) {
				None => {},
				Some(signed_msg) => {
					// Don't sign again if we already signed it
					if signed_msg.contains_signature(&(auth_index as u32)) {
						log::warn!(target:"thea","Next outgoing nonce for network {:?} is: {:?} is already signed ",network, next_outgoing_nonce);
						continue;
					}
				},
			}
			let message = match <OutgoingMessages<T>>::get(network, next_outgoing_nonce) {
				None => continue,
				Some(msg) => msg,
			};

			match message.payload_type {
				PayloadType::ScheduledRotateValidators => {
					log::warn!(target: "thea", "Ignoring ScheduledRotateValidators message for thea");
				}
				PayloadType::ValidatorsRotated => {
					// if its validator rotated, then only the previous set should sign it.
					let (prev_auth_index, prev_signer) = prev_available_keys.first()
						.ok_or("No active keys available from previous set to sign rotation message")?;
					log::info!(target: "thea", "Previous Auth Index {:?} previous signer {:?}", prev_auth_index, prev_signer.clone());

					let msg_hash = sp_io::hashing::sha2_256(message.encode().as_slice());
					// Note: this is a double hash signing
					let signature =
						sp_io::crypto::ecdsa_sign_prehashed(THEA, &prev_signer.clone().into(), &msg_hash)
							.ok_or("Expected signature to be returned")?;
					signed_messages.push((network, next_outgoing_nonce, signature.into()));
					id = id_prev; // We need to set the id to prev for unsigned validation to pass
					auth_index = *prev_auth_index; // We need to set the id to prev for unsigned validation to pass
				}
				PayloadType::L1Deposit => {
					let msg_hash = sp_io::hashing::sha2_256(message.encode().as_slice());
					// Note: this is a double hash signing
					let signature =
						sp_io::crypto::ecdsa_sign_prehashed(THEA, &signer.clone().into(), &msg_hash)
							.ok_or("Expected signature to be returned")?;
					signed_messages.push((network, next_outgoing_nonce, signature.into()));
				}
			}

		}

		if !signed_messages.is_empty() {
			//	we batch these signatures into a single extrinsic and submit on-chain
			if let Err(()) = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(
				Call::<T>::submit_signed_outgoing_messages {
					auth_index: auth_index as u32,
					id,
					signatures: signed_messages,
				}
				.into(),
			) {
				log::error!(target:"thea","Error submitting thea unsigned txn");
			}
		}

		log::debug!(target:"thea","Thea offchain worker exiting..");
		Ok(())
	}
}
