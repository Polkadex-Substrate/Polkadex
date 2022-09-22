use std::marker::PhantomData;
use std::sync::Arc;
use sc_client_api::{Backend, BlockchainEvents, FinalityNotification, FinalityNotifications, Finalizer, HeaderBackend};
use sp_api::{ ProvideRuntimeApi, HeaderT};
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::{Block};
use ocex_primitives::OcexApi;
use futures::{StreamExt, FutureExt};


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
	finality_notifications: FinalityNotifications<B>,
	/// Local key store
	keystore: Option<SyncCryptoStorePtr>,
	_be: PhantomData<BE>,
}


impl<B, C, BE, R> OCEXWorker<B, C, BE, R>	where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: OcexApi<B>,
{
	pub(crate) fn new(params: OCEXParams<C,BE,R>) -> Self {
		let finality_notifications = params.client.finality_notification_stream();
		OCEXWorker {
			_client: params.client,
			backend: params.backend,
			runtime: params.runtime,
			finality_notifications,
			keystore: params.key_store,
			_be: Default::default()
		}
	}

	fn handle_finality_notification(&self, notification: FinalityNotification<B>){
		log::warn!(target:"ocex","New finality event: {:?}", notification.header.number());
	}

	pub async fn run(&mut self) {
		loop {
			futures::select! {
				notification = self.finality_notifications.next().fuse() => {
					if let Some(notification) = notification {
						self.handle_finality_notification(notification);
					} else {
						return;
					}
				},
			}
		}
	}
}