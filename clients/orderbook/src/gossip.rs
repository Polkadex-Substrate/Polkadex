use log::info;
use orderbook_primitives::{types::GossipMessage, SnapshotSummary};
use parity_scale_codec::{Decode, Encode};
use parking_lot::RwLock;
use polkadex_primitives::AccountId;
use sc_network::PeerId;
use sc_network_common::protocol::role::ObservedRole;
use sc_network_gossip::{MessageIntent, ValidationResult, Validator, ValidatorContext};
use sp_runtime::traits::{Block, Hash, Header};
use std::{
	collections::{BTreeSet, HashMap},
	ops::Sub,
	sync::Arc,
};
use tokio::time::{Duration, Instant};

pub const REBROADCAST_INTERVAL: Duration = Duration::from_secs(3);
pub const WANT_REBROADCAST_INTERVAL: Duration = Duration::from_secs(3);

/// Gossip engine messages topic
pub fn topic<B: Block>() -> B::Hash
where
	B: Block,
{
	<<B::Header as Header>::Hashing as Hash>::hash(b"orderbook")
}

/// Orderbook gossip validator
///
/// Validate Orderbook gossip messages and limit the number of broadcast rounds.
///
/// Allows messages for 'rounds >= last concluded' to flow, everything else gets
/// rejected/expired.
///
///All messaging is handled in a single Orderbook global topic.
pub struct GossipValidator<B>
where
	B: Block,
{
	_topic: B::Hash,
	latest_worker_nonce: Arc<RwLock<u64>>,
	last_snapshot: Arc<RwLock<SnapshotSummary<AccountId>>>,
	is_validator: bool,
	pub(crate) fullnodes: Arc<RwLock<BTreeSet<PeerId>>>,
	pub(crate) message_cache: Arc<RwLock<HashMap<([u8; 16], PeerId), Instant>>>,
}

impl<B> GossipValidator<B>
where
	B: Block,
{
	pub fn new(
		latest_worker_nonce: Arc<RwLock<u64>>,
		fullnodes: Arc<RwLock<BTreeSet<PeerId>>>,
		is_validator: bool,
		last_snapshot: Arc<RwLock<SnapshotSummary<AccountId>>>,
	) -> GossipValidator<B> {
		GossipValidator {
			_topic: topic::<B>(),
			latest_worker_nonce,
			fullnodes,
			is_validator,
			last_snapshot,
			message_cache: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	pub fn validate_message(
		&self,
		message: &GossipMessage,
		peerid: PeerId,
	) -> ValidationResult<B::Hash> {
		let msg_hash = sp_core::hashing::blake2_128(&message.encode());
		// Discard if we already know this message
		if self.message_cache.read().contains_key(&(msg_hash, peerid)) {
			return ValidationResult::Discard
		}
		match message {
			GossipMessage::ObMessage(msg) => {
				let latest_worker_nonce = *self.latest_worker_nonce.read();
				if msg.worker_nonce > latest_worker_nonce {
					// It's a new message so we process it and keep it in our pool
					ValidationResult::ProcessAndKeep(topic::<B>())
				} else {
					// We already saw this message, so discarding.
					ValidationResult::Discard
				}
			},

			GossipMessage::WantWorkerNonce(from, to) => {
				if from > to {
					// Invalid request
					return ValidationResult::Discard
				}
				// Validators only process it if the request is for nonces after
				if *from >= self.last_snapshot.read().worker_nonce {
					ValidationResult::ProcessAndKeep(topic::<B>())
				} else {
					ValidationResult::Discard
				}
			},
			GossipMessage::Want(snapshot_id, _) => {
				if self.is_validator {
					// Only fullnodes will respond to this
					return ValidationResult::Discard
				}
				// We only process the request for last snapshot
				if self.last_snapshot.read().snapshot_id == *snapshot_id {
					self.message_cache.write().insert((msg_hash, peerid), Instant::now());
					ValidationResult::ProcessAndKeep(topic::<B>())
				} else {
					ValidationResult::Discard
				}
			},
			_ => {
				// Rest of the match patterns are directed messages so we assume that directed
				// messages are only accessible to those recipient peers so we process and
				// discard them and not propagate to others
				if self.message_cache.read().contains_key(&(msg_hash, peerid)) {
					ValidationResult::Discard
				} else {
					self.message_cache.write().insert((msg_hash, peerid), Instant::now());
					ValidationResult::ProcessAndDiscard(topic::<B>())
				}
			},
		}
	}

	/// Returns true if the messgae can be rebroadcasted
	pub fn rebroadcast_check(&self, message: &GossipMessage, peerid: PeerId) -> bool {
		let msg_hash = sp_core::hashing::blake2_128(&message.encode());
		let interval = match message {
			GossipMessage::Want(_, _) => WANT_REBROADCAST_INTERVAL,
			_ => REBROADCAST_INTERVAL,
		};
		if self.message_expired_check(message) {
			return false
		}
		match self.message_cache.read().get(&(msg_hash, peerid)) {
			None => true,
			Some(last_time) => Instant::now().sub(*last_time) > interval,
		}
	}

	/// Returns true if the message is expired.
	pub fn message_expired_check(&self, message: &GossipMessage) -> bool {
		match message {
			GossipMessage::ObMessage(msg) =>
				msg.worker_nonce < self.last_snapshot.read().worker_nonce,

			GossipMessage::WantWorkerNonce(from, _) => {
				// Validators only process it if the request is for nonces after
				*from < self.last_snapshot.read().worker_nonce
			},

			GossipMessage::Want(snapshot_id, _) =>
				*snapshot_id != self.last_snapshot.read().snapshot_id,
			_ => false,
		}
	}
}

impl<B> Validator<B> for GossipValidator<B>
where
	B: Block,
{
	fn new_peer(&self, _context: &mut dyn ValidatorContext<B>, who: &PeerId, role: ObservedRole) {
		info!(target:"orderbook","New peer connected: {:?}, role: {:?}",who,role);

		if let ObservedRole::Full = role {
			self.fullnodes.write().insert(*who);
		}
	}
	fn peer_disconnected(&self, _context: &mut dyn ValidatorContext<B>, who: &PeerId) {
		info!(target:"orderbook","New peer disconnected: {:?}",who);
		self.fullnodes.write().remove(who);
	}
	fn validate(
		&self,
		_context: &mut dyn ValidatorContext<B>,
		sender: &PeerId,
		mut data: &[u8],
	) -> ValidationResult<B::Hash> {
		// Decode
		if let Ok(ob_message) = GossipMessage::decode(&mut data) {
			// Check if we processed this message
			let result = self.validate_message(&ob_message, *sender);
			match result {
				ValidationResult::ProcessAndKeep(_) =>
					info!(target:"gossip","{ob_message:?} validation result: P&K"),
				ValidationResult::ProcessAndDiscard(_) =>
					info!(target:"gossip","{ob_message:?} validation result: P&D"),
				ValidationResult::Discard =>
					info!(target:"gossip","{ob_message:?} validation result: D"),
			}
			return result
		}
		ValidationResult::Discard
	}

	fn message_expired<'a>(&'a self) -> Box<dyn FnMut(B::Hash, &[u8]) -> bool + 'a> {
		Box::new(move |_topic, mut data| {
			// Decode
			let msg = match GossipMessage::decode(&mut data) {
				Ok(msg) => msg,
				Err(_) => return true,
			};
			let result = self.message_expired_check(&msg);
			info!(target:"gossip","{msg:?} expiry check result: {result:?}");
			result
		})
	}

	fn message_allowed<'a>(
		&'a self,
	) -> Box<dyn FnMut(&PeerId, MessageIntent, &B::Hash, &[u8]) -> bool + 'a> {
		Box::new(move |who, _intent, _topic, mut data| {
			// Decode
			let msg = match GossipMessage::decode(&mut data) {
				Ok(vote) => vote,
				Err(_) => return false,
			};
			// Logic for rebroadcasting.
			let result = self.rebroadcast_check(&msg, *who);
			info!(target:"gossip","{msg:?} egress allowed check result: {result:?}");
			result
		})
	}
}
