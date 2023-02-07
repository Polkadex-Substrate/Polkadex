use frame_support::dispatch::DispatchResult;
// Trait to add liquidity in OCEX pallet
pub trait LiquidityModifier {
	type AssetId;
	type AccountId;
	fn on_deposit(account: Self::AccountId, asset: Self::AssetId, balance: u128) -> DispatchResult;
	fn on_withdraw(
		account: Self::AccountId,
		proxy_account: Self::AccountId,
		asset: Self::AssetId,
		balance: u128,
		do_force_withdraw: bool,
	) -> DispatchResult;
	fn on_register(main_account: Self::AccountId, proxy: Self::AccountId) -> DispatchResult;
	#[cfg(feature = "runtime-benchmarks")]
	fn set_exchange_state_to_true() -> DispatchResult;
	#[cfg(feature = "runtime-benchmarks")]
	fn allowlist_and_create_token(account: Self::AccountId, token: u128) -> DispatchResult;
}
