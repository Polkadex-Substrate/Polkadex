#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;

use crate::AuthorityId;

sp_api::decl_runtime_apis! {
	/// APIs necessary for Thea in the parachain
	pub trait TheaParachainApi
	{
		/// Returns the current authority set
		fn get_current_authorities() -> Vec<AuthorityId>;
	}
}