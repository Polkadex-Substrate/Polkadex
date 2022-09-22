mod keystore;
mod error;

use std::marker::PhantomData;
use std::sync::Arc;
use sc_client_api::{Backend, BlockchainEvents, FinalityNotification, FinalityNotifications, Finalizer, HeaderBackend};
use sp_api::{ProvideRuntimeApi, HeaderT, Encode};
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::{Block};
use ocex_primitives::{AuthorityId, ConsensusLog, OCEX_ENGINE_ID, OcexApi, ValidatorSet};
use futures::{StreamExt, FutureExt};
use log::{debug, error};
use sp_runtime::codec::Codec;
use sp_runtime::generic::{BlockId, OpaqueDigestItemId};
use crate::error::Error;
use crate::keystore::OcexKeystore;


/// OCEX client
pub struct OCEXParams<C, BE, R> {
	/// THEA client
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	/// Local key store
	pub key_store: Option<SyncCryptoStorePtr>,
}

/// A convenience OCEX client trait that defines all the type bounds a THEA client
/// has to satisfy. Ideally that should actually be a trait alias. Unfortunately as
/// of today, Rust does not allow a type alias to be used as a trait bound. Tracking
/// issue is <https://github.com/rust-lang/rust/issues/41517>.
pub trait Client<B, BE>:
BlockchainEvents<B> + HeaderBackend<B> + Finalizer<B, BE> + ProvideRuntimeApi<B> + Send + Sync
	where
		B: Block,
		BE: Backend<B>,
{
	// empty
}

impl<B, BE, T> Client<B, BE> for T
	where
		B: Block,
		BE: Backend<B>,
		T: BlockchainEvents<B>
		+ HeaderBackend<B>
		+ Finalizer<B, BE>
		+ ProvideRuntimeApi<B>
		+ Send
		+ Sync,
{
	// empty
}


/// OCEX worker
pub struct OCEXWorker<B, C, BE, R>
	where
		B: Block,
		BE: Backend<B>,
		C: Client<B, BE>,
		R: ProvideRuntimeApi<B>,
		R::Api: OcexApi<B>,
{
	_client: Arc<C>,
	#[allow(dead_code)]
	backend: Arc<BE>,
	runtime: Arc<R>,
	wait_for_keys: u8,
	finality_notifications: FinalityNotifications<B>,
	/// Local key store
	keystore: OcexKeystore,
	_be: PhantomData<BE>,
}


impl<B, C, BE, R> OCEXWorker<B, C, BE, R>	where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: OcexApi<B>,
{
	pub fn new(params: OCEXParams<C,BE,R>) -> Self {
		let finality_notifications = params.client.finality_notification_stream();
		OCEXWorker {
			_client: params.client,
			backend: params.backend,
			runtime: params.runtime,
			wait_for_keys: 0,
			finality_notifications,
			keystore: params.key_store.into(),
			_be: Default::default()
		}
	}

	/// Check if we have session keys for any of the current validators
	fn has_key(&self, validator_set: &ValidatorSet<AuthorityId>) -> Option<(usize,AuthorityId)> {
		if let Some(authority_id) = self.keystore.authority_id(&validator_set.validators[..]) {
			validator_set
				.validators
				.iter()
				.position(|id| &authority_id == id)
				.map(|auth_idx| (auth_idx, authority_id))
		} else {
			None
		}
	}

	fn handle_finality_notification(&mut self, notification: FinalityNotification<B>) -> Result<bool, Error> {
		log::info!(target:"ocex","New finality event: {:?}", notification.header.number());
		let at = BlockId::hash(notification.hash);
		// Check for ocex keys in storage and if it is not found, wait for another 10 finalized blocks and panic
		if let Some(active) =self.validator_set(&notification.header) {
			if let Some((auth_index, authority)) = self.has_key(&active) {
				// TODO: read for unverified enclave reports storage
				// TODO: Verify reports
				// sign it
				let payload_to_sign = [0u8;3]; // TODO: This is a dummy payload
				let signature = self.keystore.sign(&authority, &payload_to_sign.encode())?;
				// send approvals to OCEX pallet

				match self.runtime.runtime_api().approve_enclave_report(&at, authority, signature) {
					Ok(res) => match res {
						Ok(()) => {
							debug!(target: "ocex", "successfully submitted the enclave report approval");
							// TODO: Keep track of signed reports so you don't send approvals again.
						},
						Err(err) => {
							error!(target: "ocex", "Unable to sign transaction; {:?}", err);
						},
					},
					Err(err) => {
						error!(target: "ocex", "Error in runtime api call: {:?}", err);
					},
				};
			}else {
				self.wait_for_keys = self.wait_for_keys.saturating_add(1);
				log::error!(target:"ocex", "Ocex Session keys are not inserted, validator will stop in {:?} blocks", 50u8.saturating_sub(self.wait_for_keys));
				if self.wait_for_keys == 50 {
					// We cannot proceed without keys so we are instructing the worker loop to stop
					return Ok(false)
				}
			}
		}else {
			log::debug!(target:"ocex", "OCEX pallet is not yet deployed, skipping ocex logic.")
		}
		Ok(true)
	}

	/// Return the current active validator set at header `header`.
	///
	/// Note that the validator set could be `None`. This is the case if we don't find
	/// a OCEX authority set change and we can't fetch the authority set from the
	/// on-chain state.
	///
	/// Such a failure is usually an indication that the OCEX pallet has not been deployed (yet).
	fn validator_set(&self, header: &B::Header) -> Option<ValidatorSet<AuthorityId>> {
		find_authorities_change::<B, AuthorityId>(header).or_else(|| {
			let at = BlockId::hash(header.hash());
			self.runtime.runtime_api().validator_set(&at).ok()
		})
	}

	pub async fn run(&mut self) {
		loop {
			futures::select! {
				notification = self.finality_notifications.next().fuse() => {
					if let Some(notification) = notification {
						match self.handle_finality_notification(notification) {
							Ok(flag) if !flag => break,
							Ok(_) => {},
							Err(err) => {
								log::error!(target:"ocex","Error while processing enclave report approvals: {:?}",err);
							}
						}
					}
				},
			}
		}
	}
}

/// Scan the `header` digest log for a OCEX validator set change. Return either the new
/// validator set or `None` in case no validator set change has been signaled.
pub fn find_authorities_change<B, Id>(header: &B::Header) -> Option<ValidatorSet<Id>>
	where
		B: Block,
		Id: Codec,
{
	let id = OpaqueDigestItemId::Consensus(&OCEX_ENGINE_ID);

	let filter = |log: ConsensusLog<Id>| match log {
		ConsensusLog::AuthoritiesChange(validator_set) => Some(validator_set),
		_ => None,
	};

	header.digest().convert_first(|l| l.try_to(id).and_then(filter))
}