// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
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

use crate::{Config, Pallet};
use frame_support::{log, traits::OneSessionHandler};
use sp_core::{bounded::BoundedVec, Get};
use sp_std::vec::Vec;

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::TheaId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::TheaId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::TheaId)>,
	{
		let authorities = validators.map(|(_, k)| k).collect::<Vec<_>>();
		// we panic here as runtime maintainers can simply reconfigure genesis and restart the
		// chain easily
		Self::initialize_authorities(&authorities).expect("Authorities vec too big");
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::TheaId)>,
	{
		// TODO: what happens if someone changes their session key
		// and queued_validators != validators in next set.???
		let next_authorities = validators.map(|(_, k)| k).collect::<Vec<_>>();
		if next_authorities.len() as u32 > T::MaxAuthorities::get() {
			log::error!(
				target: "runtime::thea",
				"authorities list {:?} truncated to length {}",
				next_authorities, T::MaxAuthorities::get(),
			);
		}
		let bounded_next_authorities =
			BoundedVec::<_, T::MaxAuthorities>::truncate_from(next_authorities);

		let next_queued_authorities = queued_validators.map(|(_, k)| k).collect::<Vec<_>>();
		if next_queued_authorities.len() as u32 > T::MaxAuthorities::get() {
			log::error!(
				target: "runtime::thea",
				"queued authorities list {:?} truncated to length {}",
				next_queued_authorities, T::MaxAuthorities::get(),
			);
		}
		let bounded_next_queued_authorities =
			BoundedVec::<_, T::MaxAuthorities>::truncate_from(next_queued_authorities);

		Self::change_authorities(bounded_next_authorities, bounded_next_queued_authorities);
	}

	fn on_disabled(_i: u32) {}
}
