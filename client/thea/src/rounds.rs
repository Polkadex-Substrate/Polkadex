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

use curv::elliptic::curves::{ECPoint, Secp256k1};
use std::collections::HashMap;

use log::*;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::{
	keygen::{Keygen, LocalKey, ProtocolMessage},
	sign::{CompletedOfflineStage, OfflineStage},
	traits::RoundBlame,
};
use round_based::{IsCritical, Msg, StateMachine};

use thea_primitives::ValidatorSetId;

use crate::{error, error::Error, inherents::update_shared_public_key};

pub struct RoundTracker {
	rounds: HashMap<ValidatorSetId, Round>,
	active_set_id: ValidatorSetId,
}

impl RoundTracker {
	pub(crate) fn new() -> RoundTracker {
		RoundTracker { rounds: HashMap::new(), active_set_id: 0 }
	}
	pub(crate) fn new_round(&mut self, validator_set_id: ValidatorSetId) -> Option<Round> {
		self.rounds.insert(
			validator_set_id,
			Round {
				local_party: None,
				local_key: None,
				mpc_local_key: None,
				offline_party: HashMap::new(),
				offline_stage: HashMap::new(),
				active_validator_len: 0,
				validator_idx: 0,
				keygen_not_started: true,
				keygen_completed: false,
			},
		)
	}

	pub(crate) fn set_active_round_id(&mut self, id: ValidatorSetId) {
		self.active_set_id = id;
	}

	pub(crate) fn active_round_id(&self) -> ValidatorSetId {
		self.active_set_id
	}

	/// Executes the first step of Keygen state machine
	pub(crate) fn start_keygen(
		&mut self,
		id: ValidatorSetId,
	) -> Option<(Vec<Msg<ProtocolMessage>>, u16)> {
		let local_party = self.mutable_local_party(id).ok()?;
		match local_party.proceed() {
			Ok(()) => {
				let messages = local_party.message_queue().clone();
				local_party.message_queue().clear();
				debug!(target: "thea", "successfully executed keygen round0");
				return Some((messages, local_party.current_round()))
			},
			Err(err) => {
				if err.is_critical() {
					error!(target: "thea", "Crititcal Error in MPC State machine: {:?}", err);
				}
				warn!(target: "thea", "Error reported by MPC State machine: {:?}", err)
			},
		}
		None
	}
	pub(crate) fn set_local_party(
		&mut self,
		id: &ValidatorSetId,
		local_party: Option<Keygen>,
	) -> Result<(), Error> {
		let round = self.rounds.get_mut(id).ok_or(Error::RoundNotFound(*id))?;
		round.local_party = local_party;
		Ok(())
	}

	pub(crate) fn mutable_local_party(&mut self, id: ValidatorSetId) -> Result<&mut Keygen, Error> {
		let round = self.rounds.get_mut(&id).ok_or(Error::RoundNotFound(id))?;
		round.local_party.as_mut().ok_or(Error::LocalPartyNotInitialized)
	}

	pub(crate) fn local_party(&self, id: ValidatorSetId) -> Result<&Keygen, Error> {
		let round = self.rounds.get(&id).ok_or(Error::RoundNotFound(id))?;
		round.local_party.as_ref().ok_or(Error::LocalPartyNotInitialized)
	}

	pub(crate) fn set_offline_party(
		&mut self,
		id: &ValidatorSetId,
		offline_party: OfflineStage,
		submission_block: u32,
		payload: &Payload,
	) -> Result<(), Error> {
		let round = self.rounds.get_mut(id).ok_or(Error::RoundNotFound(*id))?;
		round.offline_party.insert(*payload, (submission_block, offline_party));
		Ok(())
	}

	pub(crate) fn submission_block(
		&self,
		id: ValidatorSetId,
		payload: &Payload,
	) -> Result<u32, Error> {
		Ok(self
			.rounds
			.get(&id)
			.ok_or(Error::RoundNotFound(id))?
			.offline_party
			.get(payload)
			.ok_or(Error::OfflinePartyNotInitialized)?
			.0)
	}

	pub(crate) fn mutable_offline_party(
		&mut self,
		id: ValidatorSetId,
		payload: &Payload,
	) -> Result<&mut OfflineStage, Error> {
		Ok(&mut self
			.rounds
			.get_mut(&id)
			.ok_or(Error::RoundNotFound(id))?
			.offline_party
			.get_mut(payload)
			.ok_or(Error::OfflinePartyNotInitialized)?
			.1)
	}

	pub(crate) fn offline_party(
		&self,
		id: ValidatorSetId,
		payload: &Payload,
	) -> Result<&OfflineStage, Error> {
		let round = self.rounds.get(&id).ok_or(Error::RoundNotFound(id))?;
		match &round.offline_party.get(payload) {
			Some(p) => Ok(&p.1),
			None => Err(Error::RoundNotFound(id)),
		}
	}

	pub(crate) fn is_finished(&self, id: ValidatorSetId) -> Result<bool, Error> {
		Ok(self.local_party(id)?.is_finished())
	}

	pub(crate) fn is_offline_finished(
		&self,
		id: ValidatorSetId,
		payload: &Payload,
	) -> Result<bool, Error> {
		Ok(self.offline_party(id, payload)?.is_finished())
	}

	pub(crate) fn pick_output(&mut self, id: ValidatorSetId) -> Result<LocalKey<Secp256k1>, Error> {
		Ok(self
			.mutable_local_party(id)?
			.pick_output()
			.ok_or(Error::ProtocolNotComplete)??)
	}

	pub(crate) fn pick_offline_output(
		&mut self,
		id: ValidatorSetId,
		payload: &Payload,
	) -> Result<CompletedOfflineStage, Error> {
		Ok(self
			.mutable_offline_party(id, payload)?
			.pick_output()
			.ok_or(Error::ProtocolNotComplete)??)
	}

	pub fn keygen_status(&self, id: ValidatorSetId) -> Result<(u16, Vec<u16>), Error> {
		let round = self.rounds.get(&id).ok_or(Error::RoundNotFound(id))?;
		let local_party = round.local_party.as_ref().ok_or(Error::LocalPartyNotInitialized)?;
		Ok(local_party.round_blame())
	}

	pub fn offline_status(
		&self,
		id: ValidatorSetId,
		payload: &Payload,
	) -> Result<(u16, Vec<u16>), Error> {
		let round = self.rounds.get(&id).ok_or(Error::RoundNotFound(id))?;
		Ok(round
			.offline_party
			.get(payload)
			.ok_or(Error::OfflinePartyNotInitialized)?
			.1
			.round_blame())
	}

	pub fn current_round(&self, id: ValidatorSetId) -> Result<u16, Error> {
		let round = self.rounds.get(&id).ok_or(Error::RoundNotFound(id))?;
		let local_party = round.local_party.as_ref().ok_or(Error::LocalPartyNotInitialized)?;
		Ok(local_party.current_round())
	}

	pub fn offline_current_round(
		&self,
		id: ValidatorSetId,
		payload: &Payload,
	) -> Result<u16, Error> {
		let round = self.rounds.get(&id).ok_or(Error::RoundNotFound(id))?;
		Ok(round
			.offline_party
			.get(payload)
			.ok_or(Error::RoundNotFound(id))?
			.1
			.current_round())
	}

	pub(crate) fn set_local_key(
		&mut self,
		id: ValidatorSetId,
		local_key: LocalKey<Secp256k1>,
	) -> Result<(), Error> {
		let round = self.rounds.get_mut(&id).ok_or(Error::RoundNotFound(id))?;
		match sp_core::ecdsa::Public::from_full(
			&local_key.public_key().as_raw().serialize_compressed(),
		) {
			Ok(ecdsa_pubk) => {
				round.set_local_key(ecdsa_pubk.clone());
				round.set_mpc_local_key(local_key);
				if update_shared_public_key(id, ecdsa_pubk).is_some() {
					warn!(target: "thea", "ECDSA Public key already existed for given set_id: {:?}",id);
				};
			},
			Err(err) => {
				error!(target: "thea","Unable to convert to compressed ecdsa public key: {:?}",err);
			},
		}
		Ok(())
	}

	pub(crate) fn set_completed_offline_stage(
		&mut self,
		id: ValidatorSetId,
		completed_offline_stage: CompletedOfflineStage,
		payload: &Payload,
	) -> Result<(), Error> {
		let round = self.rounds.get_mut(&id).ok_or(Error::RoundNotFound(id))?;
		round.set_completed_offline_stage(completed_offline_stage, payload);
		Ok(())
	}

	pub(crate) fn get_completed_offline_stage(
		&self,
		id: ValidatorSetId,
		payload: &Payload,
	) -> Result<CompletedOfflineStage, Error> {
		let round = self.rounds.get(&id).ok_or(Error::RoundNotFound(id))?;
		round.get_completed_offline_stage(payload).ok_or(Error::LocalKeyNotReady)
	}

	pub(crate) fn rng_increment(&mut self, id: &ValidatorSetId, payload: &Payload) -> u64 {
		let mut not = 0;
		let counter: &mut u64 = match self.rounds.get_mut(id) {
			Some(os) => match os.offline_stage.get_mut(payload) {
				Some(oss) => &mut oss.2,
				None => &mut not,
			},
			None => &mut not,
		};
		*counter += 1;
		*counter
	}

	pub(crate) fn get_local_key(
		&self,
		id: ValidatorSetId,
	) -> Result<sp_core::ecdsa::Public, Error> {
		let round = self.rounds.get(&id).ok_or(Error::RoundNotFound(id))?;
		round.get_local_key().ok_or(Error::LocalKeyNotReady)
	}

	pub(crate) fn get_mpc_local_key(
		&self,
		id: ValidatorSetId,
	) -> Result<LocalKey<Secp256k1>, Error> {
		let round = self.rounds.get(&id).ok_or(Error::RoundNotFound(id))?;
		round.get_mpc_local_key().ok_or(Error::LocalKeyNotReady)
	}

	// TODO @ZK use generics for these getter functions
	pub fn set_active_validator_len(
		&mut self,
		id: &ValidatorSetId,
		length: usize,
	) -> Result<(), Error> {
		let round = self.rounds.get_mut(id).ok_or(Error::RoundNotFound(*id))?;
		round.active_validator_len = length;
		Ok(())
	}

	pub fn set_validator_idx(
		&mut self,
		id: &ValidatorSetId,
		validator_idx: u16,
	) -> Result<(), Error> {
		let round = self.rounds.get_mut(id).ok_or(Error::RoundNotFound(*id))?;
		round.validator_idx = validator_idx;
		Ok(())
	}

	pub fn set_keygen_not_started(
		&mut self,
		id: &ValidatorSetId,
		keygen_not_started: bool,
	) -> Result<(), Error> {
		let round = self.rounds.get_mut(id).ok_or(Error::RoundNotFound(*id))?;
		round.keygen_not_started = keygen_not_started;
		Ok(())
	}

	pub fn set_keygen_completed(
		&mut self,
		id: &ValidatorSetId,
		keygen_completed: bool,
	) -> Result<(), Error> {
		let round = self.rounds.get_mut(id).ok_or(Error::RoundNotFound(*id))?;
		round.keygen_completed = keygen_completed;
		Ok(())
	}

	pub fn set_offline_stage_started(
		&mut self,
		id: &ValidatorSetId,
		offline_stage_started: bool,
		payload: &Payload,
	) -> Result<(), Error> {
		let round = self.rounds.get_mut(id).ok_or(Error::RoundNotFound(*id))?;
		if let Some(os) = round.offline_stage.get_mut(payload) {
			os.0 = offline_stage_started;
		}
		Ok(())
	}

	pub fn set_offline_stage_completed(
		&mut self,
		id: &ValidatorSetId,
		offline_stage_completed: bool,
		payload: &Payload,
	) -> Result<(), Error> {
		let round = self.rounds.get_mut(id).ok_or(Error::RoundNotFound(*id))?;
		if let Some(os) = round.offline_stage.get_mut(payload) {
			os.1 = offline_stage_completed;
		}
		Ok(())
	}

	pub fn get_active_validator_len(&self, id: &ValidatorSetId) -> Result<usize, Error> {
		let round = self.rounds.get(id).ok_or_else(|| Error::RoundNotFound(*id))?;
		Ok(round.active_validator_len)
	}

	pub fn get_validator_idx(&self, id: &ValidatorSetId) -> Result<u16, Error> {
		let round = self.rounds.get(id).ok_or_else(|| Error::RoundNotFound(*id))?;
		Ok(round.validator_idx)
	}

	pub fn get_keygen_not_started(&self, id: &ValidatorSetId) -> bool {
		if let Some(round) = self.rounds.get(id) {
			round.keygen_not_started
		} else {
			true
		}
	}

	pub fn get_keygen_completed(&self, id: &ValidatorSetId) -> Result<bool, Error> {
		let round = self.rounds.get(id).ok_or(Error::RoundNotFound(*id))?;
		Ok(round.keygen_completed)
	}

	pub fn get_offline_party(&self, id: &ValidatorSetId) -> Result<Vec<Payload>, Error> {
		let round = self.rounds.get(id).ok_or_else(|| Error::RoundNotFound(*id))?;
		Ok(round.offline_party.iter().map(|(k, _)| *k).collect())
	}

	pub fn get_offline_stage_started(
		&self,
		id: &ValidatorSetId,
		payload: &Payload,
	) -> Result<bool, Error> {
		let round = self.rounds.get(id).ok_or(Error::RoundNotFound(*id))?;
		match round.offline_stage.get(payload) {
			Some(rnd) => Ok(rnd.0),
			// required for new stage
			None => Ok(false),
		}
	}

	pub fn get_offline_stage_completed(
		&self,
		id: &ValidatorSetId,
		payload: &Payload,
	) -> Result<bool, Error> {
		let round = self.rounds.get(id).ok_or(Error::RoundNotFound(*id))?;
		match round.offline_stage.get(payload) {
			Some(rnd) => Ok(rnd.1),
			// required for new stage
			None => Ok(false),
		}
	}

	pub(crate) fn remove_offline_stage(&mut self, validator_set_id: &u64, payload: &[u8; 32]) {
		if let Some(round) = self.rounds.get_mut(validator_set_id) {
			if round.remove_offline_stage(payload).is_some() {
				debug!(target: "thea", "Removed offline stage for: {:?}", payload);
			} else {
				debug!(target: "thea", "Tried to remove not existing offline stage for: {:?}", payload);
			}
		}
	}

	pub(crate) fn remove_completed_stage(&mut self, validator_set_id: &u64, payload: &[u8; 32]) {
		if let Some(round) = self.rounds.get_mut(validator_set_id) {
			if round.remove_completed_stage(payload) {
				debug!(target: "thea", "Removed offline stage for: {:?}", payload);
			} else {
				debug!(target: "thea", "Tried to remove not existing offline stage for: {:?}", payload);
			}
		}
	}
}

pub type Payload = [u8; 32];

pub(crate) struct Round {
	/// mpe instance for current round
	local_party: Option<Keygen>,
	/// Generated local key for current round
	local_key: Option<sp_core::ecdsa::Public>,
	/// MPC local_key,
	mpc_local_key: Option<LocalKey<Secp256k1>>,
	/// Offline Party
	offline_party: HashMap<Payload, (u32, OfflineStage)>,
	/// Completed Offline Stage    started completed msg_count
	offline_stage: HashMap<Payload, (bool, bool, u64, CompletedOfflineStage)>,
	/// To be Documented
	active_validator_len: usize,
	validator_idx: u16,
	keygen_not_started: bool,
	keygen_completed: bool,
}

impl Round {
	fn remove_offline_stage(&mut self, payload: &Payload) -> Option<bool> {
		Some(self.offline_stage.remove(payload)?.0)
	}

	fn remove_completed_stage(&mut self, payload: &Payload) -> bool {
		self.offline_party.remove(payload).is_some()
	}

	pub(crate) fn set_local_key(&mut self, local_key: sp_core::ecdsa::Public) {
		self.local_key = Some(local_key)
	}

	pub(crate) fn set_mpc_local_key(&mut self, local_key: LocalKey<Secp256k1>) {
		self.mpc_local_key = Some(local_key)
	}

	pub(crate) fn set_completed_offline_stage(
		&mut self,
		completed_offline: CompletedOfflineStage,
		payload: &Payload,
	) {
		let old = match self.offline_stage.get(payload) {
			Some((_, _, old, _)) => *old,
			None => 0_u64,
		};
		self.offline_stage.insert(*payload, (true, true, old, completed_offline));
	}

	pub(crate) fn get_local_key(&self) -> Option<sp_core::ecdsa::Public> {
		self.local_key.clone()
	}

	pub(crate) fn get_mpc_local_key(&self) -> Option<LocalKey<Secp256k1>> {
		self.mpc_local_key.clone()
	}

	pub(crate) fn get_completed_offline_stage(
		&self,
		payload: &Payload,
	) -> Option<CompletedOfflineStage> {
		Some(self.offline_stage.get(payload)?.3.clone())
	}
}
