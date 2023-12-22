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


// Host functions for Thea's frost based implementation.
use sp_runtime_interface::runtime_interface;

#[cfg(feature = "std")]
use frost_secp256k1 as frost;

#[runtime_interface]
pub trait TheaFrostExt {
    fn dkg_part1(participant_identifier: u16, max_signers: u16, min_signers: u16){
        todo!()
    }
    fn dkg_part2(){
        todo!()
    }

    fn dkg_part3(){

    }

    fn nonce_commit() {

    }

    fn sign() {

    }

    fn aggregate() {

    }

    fn verify_signature() {

    }
}