#![deny(unused_crate_dependencies)]
//! Substrate Node Template CLI library.
#![warn(missing_docs)]
mod chain_spec;
#[macro_use]
mod service;
mod benchmarking;
mod cli;
mod command;
mod rpc;

// TODO: Remove this when libp2p enforces correct snow version
use snow as _;

fn main() -> sc_cli::Result<()> {
	command::run()
}
