//! Orderbook gadget specific errors
//!
//! Used for Orderbook gadget interal error handling only

use std::fmt::Debug;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum Error {
	#[error("Backend: {0}")]
	Backend(String),
	#[error("Keystore error: {0}")]
	Keystore(String),
	#[error("Signature error: {0}")]
	Signature(String),
	#[error("Session uninitialized")]
	UninitSession,
}
