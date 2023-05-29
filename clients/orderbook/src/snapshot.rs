use serde::{Deserialize, Serialize};
use serde_with::{json::JsonString, serde_as};
use std::collections::{BTreeMap, HashMap};

/// This is a dummy struct used to serialize memory db
/// We cannot serialize the hashmap below because of non-string type in key.
#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct SnapshotStore {
	#[serde_as(as = "JsonString<Vec<(JsonString, _)>>")]
	pub map: BTreeMap<[u8; 32], (Vec<u8>, i32)>,
}

impl SnapshotStore {
	pub fn new<T: IntoIterator<Item = ([u8; 32], (Vec<u8>, i32))>>(iter: T) -> Self {
		Self { map: BTreeMap::from_iter(iter) }
	}

	pub fn to_hashmap(self) -> HashMap<[u8; 32], (Vec<u8>, i32)> {
		HashMap::from_iter(self.map.into_iter())
	}
}

#[test]
pub fn test_snapshot_deterministic_serialization() {
	// The snapshot generate same data on serialization irrespective
	// of internal map's iter()'s behaviour

	let mut map1 = HashMap::new();

	map1.insert([0; 32], (Vec::new(), 1));
	map1.insert([1; 32], (Vec::new(), 1));
	map1.insert([2; 32], (Vec::new(), 1));

	let mut map2 = HashMap::new();

	map2.insert([1; 32], (Vec::new(), 1));
	map2.insert([0; 32], (Vec::new(), 1));
	map2.insert([2; 32], (Vec::new(), 1));

	let store1 = SnapshotStore::new(map1.into_iter());
	let store2 = SnapshotStore::new(map2.into_iter());

	let data1 = serde_json::to_vec(&store1).unwrap();
	let data2 = serde_json::to_vec(&store2).unwrap();

	assert_eq!(data1, data2);
}
