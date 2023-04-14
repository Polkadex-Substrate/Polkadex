use jsonrpsee::{core::client::ClientT, rpc_params, ws_client::WsClientBuilder};
use orderbook_primitives::types::{ObMessage, UserActions};
use sp_core::Pair;

#[tokio::main]
async fn main() {
	let url = String::from("ws://localhost:9944");

	let client = WsClientBuilder::default().build(&url).await.unwrap();

	let pair = sp_core::ecdsa::Pair::from_seed(
		&[210,47, 74, 146, 227, 46, 118, 182, 126, 97, 175, 159,
			118, 93, 56, 117, 19, 19, 42, 169, 155, 14, 122, 149, 1,
			123, 228, 3, 109, 30, 110, 21]);
	// Public key is 0x02a7d451190f72881cd92a044127adf6417b788e5118f4934484fe4d789860eb33
	println!("public: {:?}", pair.public());

	let mut message = ObMessage {
		stid: 2,
		action: UserActions::BlockImport(25),
		signature: Default::default()
	};

	message.signature = pair.sign_prehashed(&message.sign_data());

	client.request("ob_submitAction", rpc_params![message]).await.unwrap()
}
