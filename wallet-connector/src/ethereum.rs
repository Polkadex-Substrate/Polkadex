use sp_std::vec::Vec;

#[cfg(feature = "std")]
use ethers_contract::EthAbiType;
#[cfg(feature = "std")]
use ethers::prelude::Eip712;
#[cfg(feature = "std")]
use ethers::types::transaction::eip712::Eip712;

use sp_runtime_interface::runtime_interface;

#[derive(Debug, Clone, Eip712, EthAbiType)]
#[eip712(
	name = "Polkadex Transaction",
	version = "3",
	chain_id = 1,
	verifying_contract = "0x0000000000000000000000000000000000000001",
	salt = "polkadex"
)]
#[cfg(feature = "std")]
pub struct EthereumSignerPayload {
	pub transaction: String,
}

/// Converts the inner call payload to hash that was used for signing
pub fn ethereum_signing(payload: &[u8]) -> Vec<u8> {
	let payload: Vec<u8> = if payload.len() > 256 {
		sp_io::hashing::blake2_256(payload).to_vec()
	} else {
		payload.to_vec()
	};
	ethereum_signer::encode(&payload[..])
}

#[runtime_interface]
pub trait EthereumSigner {
	fn encode(inner_call_payload: &[u8]) -> Vec<u8> {
		let eth_signing_payload = EthereumSignerPayload {
			transaction: "0x".to_owned() + &hex::encode(inner_call_payload),
		};

		if let Ok(hash) = eth_signing_payload.encode_eip712() {
			hash.to_vec()
		} else {
			Vec::new()
		}
	}
}
