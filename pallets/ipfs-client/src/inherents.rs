use offchain_ipfs_primitives::inherents::{INHERENT_IDENTIFIER, InherentError, InherentType};
use sp_inherents::{
    InherentData, InherentDataProvider as InherentDataProviderTrait, InherentIdentifier,
};
use log::*;
use ipfs_embed::Cid;
use lazy_static::lazy_static;
use std::sync::Arc;
use parking_lot::Mutex;

lazy_static! {
    static ref INHERENT_DATA_STORAGE: Arc<Mutex<InherentDataProvider>> =
        Arc::new(Mutex::new(InherentDataProvider::new()));
}

#[derive(Debug, Clone, Default)]
pub struct InherentDataProvider {
    pub(crate) approved_cid: Cid,
}

impl InherentDataProvider {
    pub fn new() -> Self {
        Self {
            approved_cid: Cid::default(),
        }
    }

    pub fn update_approved_cid(&mut self, cid: Cid) {
        self.approved_cid = cid
    }
}

#[async_trait::async_trait]
impl InherentDataProviderTrait for InherentDataProvider {
    fn provide_inherent_data(
        &self,
        inherent_data: &mut InherentData,
    ) -> Result<(), sp_inherents::Error> {
        inherent_data.put_data(INHERENT_IDENTIFIER, &self.approved_cid.clone())
    }

    /// When validating the inherents, the runtime implementation can throw errors. We support
    /// two error modes, fatal and non-fatal errors. A fatal error means that the block is invalid
    /// and this function here should return `Err(_)` to not import the block. Non-fatal errors
    /// are allowed to be handled here in this function and the function should return `Ok(())`
    /// if it could be handled. A non-fatal error is for example that a block is in the future
    /// from the point of view of the local node. In such a case the block import for example
    /// should be delayed until the block is valid.
    ///
    /// If this functions returns `None`, it means that it is not responsible for this error or
    /// that the error could not be interpreted.
    async fn try_handle_error(
        &self,
        identifier: &InherentIdentifier,
        error: &[u8],
    ) -> Option<Result<(), sp_inherents::Error>> {
        // Check if this error belongs to us.
        if *identifier != INHERENT_IDENTIFIER {
            return None;
        }

        match InherentError::try_from(&INHERENT_IDENTIFIER, error)? {
            InherentError::InvalidCID(wrong_cid) => {
                error!(target: "offchain-ipfs", "Invalid Cid: {:?} in Imported Block",wrong_cid);
                Some(Err(sp_inherents::Error::Application(Box::from(
                    InherentError::InvalidCID(wrong_cid),
                ))))
            }
            InherentError::WrongInherentCall => {
                error!(target: "offchain-ipfs", "Invalid Call inserted in block");
                Some(Err(sp_inherents::Error::Application(Box::from(
                    InherentError::WrongInherentCall,
                ))))
            }
        }
    }
}



pub fn get_ipfs_inherent_data() -> InherentDataProvider {
    INHERENT_DATA_STORAGE.lock().clone()
}

pub fn update_approved_cid(cid: Cid) {
    INHERENT_DATA_STORAGE.lock().update_approved_cid(cid)
}
