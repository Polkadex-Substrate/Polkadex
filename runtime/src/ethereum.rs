use sp_std::vec::Vec;

#[cfg(feature = "std")]
use ethers_contract::EthAbiType;
#[cfg(feature = "std")]
use ethers_core::types::transaction::eip712::Eip712;
#[cfg(feature = "std")]
use ethers_derive_eip712::*;
use sp_runtime_interface::runtime_interface;

#[cfg(feature = "std")]
#[derive(Debug, Clone, Eip712, EthAbiType)]
#[eip712(
	name = "Polkadex Transaction",
	version = "1",
	chain_id = 1,
	verifying_contract = "0x0000000000000000000000000000000000000001"
)]
pub struct EthereumSignerPayload {
	pub transaction: String,
}

/// Converts the inner call payload to hash that was used for signing
pub fn ethereum_signing(payload: &[u8]) -> Vec<u8> {
	ethereum_signer::encode(payload)
}

#[runtime_interface]
pub trait EthereumSigner {
	fn encode(inner_call_payload: &[u8]) -> Vec<u8> {
		let eth_signing_payload =
			EthereumSignerPayload { transaction: hex::encode(inner_call_payload) };

		// TODO: Hopefully this should never panic but need to see how to handle this.
		let hash: [u8; 32] = eth_signing_payload.encode_eip712().unwrap();
		hash.to_vec()
	}
}
