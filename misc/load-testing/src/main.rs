use jsonrpsee::{core::client::ClientT, rpc_params, ws_client::WsClientBuilder};
use orderbook_primitives::types::{
	ObMessage, ObRecoveryState, UserActions, WithdrawPayloadCallByUser, WithdrawalRequest,
};
use polkadex_primitives::{AccountId, AssetId, Signature};
use sp_core::{Encode, Pair};

#[tokio::main]
async fn main() {
	let url = String::from("ws://localhost:9945");

	let client = WsClientBuilder::default().build(&url).await.unwrap();

	let pair = sp_core::ecdsa::Pair::from_seed(&[
		210, 47, 74, 146, 227, 46, 118, 182, 126, 97, 175, 159, 118, 93, 56, 117, 19, 19, 42, 169,
		155, 14, 122, 149, 1, 123, 228, 3, 109, 30, 110, 21,
	]);
	// Public key is 0x02a7d451190f72881cd92a044127adf6417b788e5118f4934484fe4d789860eb33
	println!("public: {:?}", pair.public());
	let main = sp_keyring::AccountKeyring::Alice;
	let proxy = sp_keyring::AccountKeyring::Bob;

	let payload = WithdrawPayloadCallByUser {
		asset_id: AssetId::Polkadex,
		amount: "1".to_string(),
		timestamp: 0,
	};

	// let mut message = ObMessage {
	// 	stid: 2,
	// 	action: UserActions::Withdraw(WithdrawalRequest {
	// 		signature: Signature::from(proxy.sign(&payload.encode())),
	// 		payload,
	// 		main: AccountId::from(main.public()),
	// 		proxy: AccountId::from(proxy.public()),
	// 	}),
	// 	signature: Default::default(),
	// };

	let mut message =
		ObMessage { stid: 1, action: UserActions::BlockImport(270), signature: Default::default() };

	message.signature = pair.sign_prehashed(&message.sign_data());

	client.request("ob_submitAction", rpc_params![message]).await.unwrap()

	// let result: Vec<u8> = client.request("ob_getObRecoverState", rpc_params![]).await.unwrap();
	//
	// let result: ObRecoveryState = serde_json::from_slice(&result).unwrap();
	// println!("{:?}",result);
}
