use serde::{Deserialize, Serialize};
use serde_with::{json::JsonString, serde_as};
use std::collections::HashMap;

/// This is a dummy struct used to serialize memory db
/// We cannot serialize the hashmap below because of non-string type in key.
#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct SnapshotStore {
	#[serde_as(as = "JsonString<Vec<(JsonString, _)>>")]
	pub map: HashMap<[u8; 32], (Vec<u8>, i32)>,
}
