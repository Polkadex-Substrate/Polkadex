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

use crate::validator::map_trie_error;
use hash_db::{AsHashDB, HashDB, Prefix};
use sp_core::{Hasher, H256};
use sp_runtime::{offchain::storage::StorageValueRef, sp_std, traits::BlakeTwo256};
use sp_std::{prelude::ToOwned, vec::Vec};
use sp_trie::{trie_types::TrieDBMutBuilderV1, LayoutV1};
use trie_db::{DBValue, TrieDBMut, TrieMut};

pub struct State;

const HASHED_NULL_NODE: [u8; 31] = *b"offchain-ocex::hashed_null_node";
const NULL_NODE_DATA: [u8; 29] = *b"offchain-ocex::null_node_data";
const KEY_PREFIX: [u8; 15] = *b"offchain-ocex::";
const TRIE_ROOT: [u8; 24] = *b"offchain-ocex::trie_root";

pub struct OffchainState<'a> {
	cache: sp_std::collections::btree_map::BTreeMap<Vec<u8>, Vec<u8>>,
	trie: TrieDBMut<'a, LayoutV1<BlakeTwo256>>,
}

impl<'a> OffchainState<'a> {
	pub fn load(storage: &'a mut State, root: &'a mut H256) -> Self {
		let trie = crate::storage::get_state_trie(storage, root);
		Self { cache: Default::default(), trie }
	}

	pub fn is_empty(&self) -> bool {
		self.cache.is_empty() && self.trie.is_empty()
	}

	pub fn get(&mut self, key: &Vec<u8>) -> Result<Option<Vec<u8>>, &'static str> {
		match self.cache.get(key) {
			Some(value) => Ok(Some(value.clone())),
			None => match self.trie.get(key) {
				Err(err) => {
					log::error!(target:"ocex","Trie returned an error while get operation");
					Err(map_trie_error(err))
				},
				Ok(option) => match option {
					None => Ok(None),
					Some(value) => {
						self.cache.insert(key.clone(), value.clone());
						Ok(Some(value))
					},
				},
			},
		}
	}

	pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) {
		self.cache.insert(key, value);
	}

	pub fn commit(&mut self) -> Result<H256, &'static str> {
		for (key, value) in self.cache.iter() {
			self.trie.insert(key, value).map_err(map_trie_error)?;
		}
		self.cache.clear();
		self.trie.commit();
		Ok(*self.trie.root())
	}
}

impl State {
	fn hashed_null_node(&self) -> <BlakeTwo256 as Hasher>::Out {
		let s_r = StorageValueRef::persistent(&HASHED_NULL_NODE);
		match s_r.get::<<BlakeTwo256 as Hasher>::Out>() {
			Ok(Some(x)) => x,
			Ok(None) => {
				log::trace!(target:"ocex","hashed_null_node not found");
				BlakeTwo256::hash(&[0u8])
			},
			Err(_) => {
				log::trace!(target:"ocex","hashed_null_node get error");
				BlakeTwo256::hash(&[0u8])
			},
		}
	}

	fn null_node_data(&self) -> Vec<u8> {
		let s_r = StorageValueRef::persistent(&NULL_NODE_DATA);
		match s_r.get::<Vec<u8>>() {
			Ok(Some(x)) => x,
			Ok(None) => {
				log::trace!(target:"ocex","null_node_data is default");
				[0u8].to_vec()
			},
			Err(_) => {
				log::trace!(target:"ocex","null_node_data is default");
				[0u8].to_vec()
			},
		}
	}

	fn db_get(&self, key: &Vec<u8>) -> Option<(DBValue, i32)> {
		log::trace!(target:"ocex","Getting key: {:?}", key);
		let derive_key = self.derive_storage_key(key);
		let s_ref = StorageValueRef::persistent(derive_key.as_slice());
		match s_ref.get::<(DBValue, i32)>() {
			Ok(d) => d,
			Err(_) => None,
		}
	}

	fn db_insert(&self, key: Vec<u8>, value: (DBValue, i32)) {
		let derive_key = self.derive_storage_key(&key);
		log::trace!(target:"ocex","Inserting key: {:?}, derived: {:?}, value: {:?}", key, derive_key, value);
		let s_ref = StorageValueRef::persistent(derive_key.as_slice());
		s_ref.set(&value);
	}

	fn derive_storage_key(&self, key: &[u8]) -> Vec<u8> {
		let mut derived = KEY_PREFIX.to_vec();
		let mut cloned_key = key.to_owned();
		derived.append(&mut cloned_key);
		derived
	}
}

impl AsHashDB<BlakeTwo256, DBValue> for State {
	fn as_hash_db(&self) -> &dyn HashDB<BlakeTwo256, DBValue> {
		self
	}

	fn as_hash_db_mut<'a>(&'a mut self) -> &'a mut (dyn HashDB<BlakeTwo256, DBValue> + 'a) {
		self
	}
}

impl HashDB<BlakeTwo256, DBValue> for State {
	fn get(&self, key: &<BlakeTwo256 as Hasher>::Out, prefix: Prefix) -> Option<DBValue> {
		log::trace!(target:"ocex","HashDb get, key: {:?}, prefix: {:?}", key,prefix);
		if key == &self.hashed_null_node() {
			return Some(self.null_node_data())
		}

		let key = prefixed_key(key, prefix);
		match self.db_get(&key) {
			Some((ref d, rc)) if rc > 0 => Some(d.clone()),
			_ => None,
		}
	}

	fn contains(&self, key: &<BlakeTwo256 as Hasher>::Out, prefix: Prefix) -> bool {
		log::trace!(target:"ocex","HashDb contains, key: {:?}, prefix: {:?}", key,prefix);
		if key == &self.hashed_null_node() {
			return true
		}

		let key = prefixed_key(key, prefix);
		matches!(self.db_get(&key), Some((_, x)) if x > 0)
	}

	fn insert(&mut self, prefix: Prefix, value: &[u8]) -> <BlakeTwo256 as Hasher>::Out {
		log::trace!(target:"ocex","HashDb insert, prefix: {:?}",prefix);
		if *value == self.null_node_data() {
			return self.hashed_null_node()
		}
		let key = BlakeTwo256::hash(value);
		HashDB::emplace(self, key, prefix, value.into());
		key
	}

	fn emplace(&mut self, key: <BlakeTwo256 as Hasher>::Out, prefix: Prefix, value: DBValue) {
		log::trace!(target:"ocex","HashDb emplace, key: {:?}, prefix: {:?}", key,prefix);
		if value == self.null_node_data() {
			return
		}

		let key = prefixed_key(&key, prefix);
		match self.db_get(&key) {
			Some((mut old_value, mut rc)) => {
				if rc <= 0 {
					old_value = value;
				}
				rc += 1;
				self.db_insert(key, (old_value, rc));
			},
			None => {
				self.db_insert(key, (value, 1));
			},
		}
	}

	fn remove(&mut self, key: &<BlakeTwo256 as Hasher>::Out, prefix: Prefix) {
		log::trace!(target:"ocex","HashDb remove, key: {:?}, prefix: {:?}", key,prefix);
		if key == &self.hashed_null_node() {
			return
		}

		let key = prefixed_key(key, prefix);
		match self.db_get(&key) {
			Some((value, mut rc)) => {
				rc -= 1;
				self.db_insert(key, (value, rc));
			},
			None => {
				let value = DBValue::default();
				self.db_insert(key, (value, -1));
			},
		}
	}
}

/// Derive a database key from hash value of the polkadex-mainnet-polkadex-parachain-node (key) and
/// the polkadex-mainnet-polkadex-parachain-node prefix.
pub fn prefixed_key(key: &<BlakeTwo256 as Hasher>::Out, prefix: Prefix) -> Vec<u8> {
	let mut prefixed_key = Vec::with_capacity(key.as_ref().len() + prefix.0.len() + 1);
	prefixed_key.extend_from_slice(prefix.0);
	if let Some(last) = prefix.1 {
		prefixed_key.push(last);
	}
	prefixed_key.extend_from_slice(key.as_ref());
	prefixed_key
}

pub(crate) fn load_trie_root() -> <BlakeTwo256 as Hasher>::Out {
	let root_ref = StorageValueRef::persistent(&TRIE_ROOT);
	match root_ref.get::<<BlakeTwo256 as Hasher>::Out>() {
		Ok(Some(root)) => root,
		Ok(None) => Default::default(),
		Err(_) => Default::default(),
	}
}

pub(crate) fn store_trie_root(root: <BlakeTwo256 as Hasher>::Out) {
	let root_ref = StorageValueRef::persistent(&TRIE_ROOT);
	root_ref.set(&root);
}

pub(crate) fn get_state_trie<'a>(
	state: &'a mut State,
	root: &'a mut H256,
) -> TrieDBMut<'a, LayoutV1<BlakeTwo256>> {
	if *root == H256::zero() {
		TrieDBMutBuilderV1::new(state, root).build()
	} else {
		TrieDBMutBuilderV1::from_existing(state, root).build()
	}
}

#[cfg(test)]
mod tests {
	use trie_db::TrieMut;

	use crate::{
		mock::new_test_ext,
		storage::{get_state_trie, load_trie_root, store_trie_root, OffchainState, State},
		tests::register_offchain_ext,
	};

	#[test]
	pub fn test_commit_change_revert_pattern() {
		let mut ext = new_test_ext();
		register_offchain_ext(&mut ext);
		log::trace!(target:"ocex","test_trie_storage test starting..");
		ext.execute_with(|| {
			let mut root = load_trie_root();
			{
				let mut storage = State;

				let mut state = OffchainState::load(&mut storage, &mut root);

				state.insert(b"1".to_vec(), b"a".to_vec());
				state.insert(b"2".to_vec(), b"b".to_vec());
				state.insert(b"3".to_vec(), b"c".to_vec());

				state.commit().unwrap();
				state.insert(b"4".to_vec(), b"d".to_vec());
				state.commit().unwrap();
				state.insert(b"5".to_vec(), b"e".to_vec());
			}
			{
				let mut storage = State;

				let mut state = OffchainState::load(&mut storage, &mut root);
				state.get(&b"1".to_vec()).unwrap().unwrap();
				state.get(&b"2".to_vec()).unwrap().unwrap();
				state.get(&b"3".to_vec()).unwrap().unwrap();
				state.get(&b"4".to_vec()).unwrap().unwrap();
				assert!(state.get(&b"5".to_vec()).unwrap().is_none());
			}
		});
	}

	#[test]
	pub fn test_trie_storage() {
		let mut ext = new_test_ext();
		register_offchain_ext(&mut ext);
		log::trace!(target:"ocex","test_trie_storage test starting..");
		ext.execute_with(|| {
			let mut root = load_trie_root();
			{
				let mut storage = State;

				let mut state = get_state_trie(&mut storage, &mut root);

				state.insert(b"1", b"a").unwrap();
				state.insert(b"2", b"b").unwrap();
				state.insert(b"3", b"c").unwrap();

				state.commit();
			}

			store_trie_root(root);

			{
				let mut root = load_trie_root();
				let mut storage = State;

				let state = get_state_trie(&mut storage, &mut root);
				assert_eq!(state.get(b"1").unwrap().unwrap(), b"a");
				assert_eq!(state.get(b"2").unwrap().unwrap(), b"b");
				assert_eq!(state.get(b"3").unwrap().unwrap(), b"c");
			}
		})
	}
}
