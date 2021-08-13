use pallet_polkadex_ido_primitives::FundingRoundWithPrimitives;
use codec::Codec;
sp_api::decl_runtime_apis! {
    pub trait PolkadexIdoRuntimeApi<AccountId,Hash> where AccountId: Codec, Hash : Codec{
        fn rounds_by_investor(account : AccountId) -> Vec<(Hash, FundingRoundWithPrimitives)>;
        fn rounds_by_creator(account : AccountId) -> Vec<(Hash, FundingRoundWithPrimitives)> ;
    }
}