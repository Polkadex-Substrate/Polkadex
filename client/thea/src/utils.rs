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

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use codec::Codec;
use curv::arithmetic::Converter;
use frame_support::BoundedVec;
use log::*;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::{
	party_i::SignatureRecid, state_machine::traits::RoundBlame,
};
use round_based::{Msg, StateMachine};
use serde::{de::DeserializeOwned, Serialize};

use sp_runtime::{
	generic::OpaqueDigestItemId,
	traits::{Block, Header},
};

use thea_primitives::{
	keygen, keygen::TheaPayload, payload::SignedTheaPayload, AuthorityId, ConsensusLog, PartyIndex,
	ValidatorSet, THEA_ENGINE_ID,
};

use crate::{error, error::Error};

/// Scan the `header` digest log for a THEA validator set change. Return either the new
/// validator set or `None` in case no validator set change has been signaled.
pub fn find_authorities_change<B, Id>(header: &B::Header) -> Option<ValidatorSet<Id>>
where
	B: Block,
	Id: Codec,
{
	let id = OpaqueDigestItemId::Consensus(&THEA_ENGINE_ID);

	let filter = |log: ConsensusLog<Id>| match log {
		ConsensusLog::AuthoritiesChange(validator_set) => Some(validator_set),
		_ => None,
	};

	header.digest().convert_first(|l| l.try_to(id).and_then(filter))
}

/// Thea protocol needs threshold to be greater than 1 and less than n
/// Also, the current implementation of mpc expects threshold to be greater than 50%
pub fn threshold_parties(n: u16) -> u16 {
	(2 * n + 1) / 3
}

pub fn convert_keygen_message<S>(
	msg: &Msg<S>,
) -> Result<keygen::Msg<thea_primitives::MsgLimit>, Error>
where
	S: Serialize,
{
	if let Ok(messages) = serde_json::to_vec(&msg.body) {
		Ok(keygen::Msg {
			sender: msg.sender,
			receiver: msg.receiver,
			message: BoundedVec::try_from(messages)?,
		})
	} else {
		log::error!(target:"thea","Unable to encode keygen message, silently ignoring!");
		Err(Error::SerdeError(String::from("Unable to serialize message body")))
	}
}

pub fn convert_all_keygen_messages<S>(
	msgs: impl AsRef<[Msg<S>]>,
) -> Result<BoundedVec<keygen::Msg<thea_primitives::MsgLimit>, thea_primitives::MsgVecLimit>, Error>
where
	S: Serialize,
{
	// TODO: I don't like this solution, does anyone have a better idea?
	let messages: Vec<thea_primitives::keygen::Msg<thea_primitives::MsgLimit>> = msgs
		.as_ref()
		.iter()
		.map(convert_keygen_message)
		.filter(|res| res.is_ok())
		.map(|res| res.unwrap())
		.collect();

	Ok(BoundedVec::try_from(messages)?)
}

/// Converts a thea payload to it's original multi-party-ecdsa types
/// It will fail even if one of the messages failed to deserialize
pub fn convert_back_keygen_messages<R, S>(
	payload: TheaPayload<AuthorityId, R, thea_primitives::MsgLimit, thea_primitives::MsgVecLimit>,
) -> Result<BoundedVec<Msg<S>, thea_primitives::MsgVecLimit>, Error>
where
	R: Codec + Default,
	S: DeserializeOwned,
{
	let mut converted_msgs = vec![];
	for msg in payload.messages {
		converted_msgs.push(Msg {
			sender: msg.sender,
			receiver: msg.receiver,
			body: serde_json::from_slice(&msg.message)?, // TODO: Fix a better lifetime
		});
	}

	Ok(BoundedVec::try_from(converted_msgs)?)
}

pub fn generate_party_index_sequence(n: PartyIndex) -> Vec<PartyIndex> {
	let x = (1..n + 1).into_iter().collect();
	debug!(target:"thea", "S_l sequence: {:?}, total parties: {:?}", x, n);
	x
}

pub(crate) fn handling_incoming_messages<S, K>(
	msgs: BoundedVec<Msg<S>, thea_primitives::MsgVecLimit>,
	local_instance: &mut K,
) -> Result<(Vec<Msg<S>>, u16), Error>
where
	K: StateMachine<MessageBody = S> + RoundBlame,
	error::Error: From<<K as StateMachine>::Err>,
	S: Clone,
{
	for message in msgs {
		match message.receiver {
			// We should only process messages meant for us
			Some(receiver) if receiver == local_instance.party_ind() => {
				// This is an expensive computation
				local_instance.handle_incoming(message)?;
			},
			// or meant for everyone
			None => {
				// This is an expensive computation
				local_instance.handle_incoming(message)?;
			},
			_ => {},
		}
	}
	debug!(target: "thea", "State Machine Status(round blame): {:?}", local_instance.round_blame());
	if local_instance.wants_to_proceed() {
		// This is an expensive computation
		local_instance.proceed()?;
	}
	let messages = local_instance.message_queue().clone();
	local_instance.message_queue().clear();
	Ok((messages, local_instance.current_round()))
}

pub fn convert_signature(signature: &SignatureRecid) -> Result<sp_core::ecdsa::Signature, Error> {
	let recid = secp256k1::ecdsa::RecoveryId::from_i32(signature.recid as i32)?;
	let mut signature_template = signature.r.to_bigint().to_bytes();
	signature_template.append(&mut signature.s.to_bigint().to_bytes());
	let signature_converted =
		secp256k1::ecdsa::RecoverableSignature::from_compact(&signature_template, recid)?;

	// TODO: Inbuilt conversion is not working for some reason., copy pasted the code here
	let mut r = sp_core::ecdsa::Signature::default();
	let (recid, sig) = signature_converted.serialize_compact();
	r.0[..64].copy_from_slice(&sig);
	// This is safe due to the limited range of possible valid ids.
	r.0[64] = recid.to_i32() as u8;

	Ok(r)
}

pub fn handle_error(result: Result<(), Error>, tag: &str) {
	if let Err(err) = result {
		error!(target: "thea", "{:?}, {:?}", tag, err)
	}
}

pub fn verify_payload(
	pubk: &sp_core::ecdsa::Public,
	signed_payloads: &[SignedTheaPayload],
) -> Result<(), Error> {
	for payload in signed_payloads {
		if !thea_primitives::runtime::crypto::verify_ecdsa_prehashed(
			&payload.signature,
			pubk,
			&payload.payload.payload,
		) {
			error!(target:"thea", "Failed to verify Thea ecdsa key");
			return Err(Error::ECDSASignatureError(String::from("Signature verification Failed")));
		}
	}
	Ok(())
}
