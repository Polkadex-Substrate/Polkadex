#[cfg(feature = "std")]
use blst::min_sig::*;
#[cfg(feature = "std")]
use blst::BLST_ERROR;
use sp_runtime_interface::runtime_interface;

#[runtime_interface]
pub trait BLS {}
