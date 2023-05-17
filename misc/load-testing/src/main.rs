use jsonrpsee::{core::client::ClientT, rpc_params, ws_client::WsClientBuilder};
use orderbook_primitives::{
	recovery::ObRecoveryState,
	types::{ObMessage, Trade, UserActions, WithdrawPayloadCallByUser, WithdrawalRequest},
};
use polkadex_primitives::{AccountId, AssetId, Signature};
use sp_core::{Encode, Pair};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let url = String::from("wss://solochain-bootnode.polkadex.trade:443");

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

	for i in 1..1500 {
		//		println!("Sending {i} block");
		let mut message = ObMessage {
			stid: 1,
			worker_nonce: i,
			action: UserActions::BlockImport(i as u32),
			signature: Default::default(),
		};

		message.signature = pair.sign_prehashed(&message.sign_data());

		let _response = client.request("ob_submitAction", rpc_params![message]).await?;
	}

	let subxt = 

	let proposal = subxt.metadata().module_with_calls("OCEX")
		.and_then(|module| module.call("ocex_whitelistOrderbookOperator", "0x02a7d451190f72881cd92a044127adf6417b788e5118f4934484fe4d789860eb33"))
		.map_err(|_| Error::msg("failed to compose a sudo call"))?;
	let call = Call::new("Sudo", "sudo", proposal);

	// Trade action
	// Set exchange state as root
	// Create random asset + use polkadex
	// Register pair
	// Registr accounts OCEX -> register_main_account
	// Deposit assets OCEX -> deposit (both currencies)
	// Create random order -> engine -> random_order_for_testing
	//let mut message = ObMessage {
	//	stid: 2,
	//	action: UserActions::Trade(vec![Trade]),
	//	signature: Default::default(),
	//};

	//let result: Vec<u8> = client.request("ob_getObRecoverState", rpc_params![]).await.unwrap();
	//let result: ObRecoveryState = serde_json::from_slice(&result).unwrap();
	//println!("{:?}", result);
	Ok(())
}

//fn main() {}
