use std::marker::PhantomData;
use std::sync::Arc;
use sc_client_api::{Backend, BlockchainEvents, FinalityNotifications, Finalizer, HeaderBackend};
use sp_api::{ ProvideRuntimeApi};
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::{Block};
use ocex_primitives::OcexApi;

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
	// key_store: OCEXKeyS,
	finality_notifications: FinalityNotifications<B>,
	_be: PhantomData<BE>,
}