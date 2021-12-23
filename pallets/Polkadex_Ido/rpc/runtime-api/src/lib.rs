#![cfg_attr(not(feature = "std"), no_std)]
use codec::Codec;
use pallet_polkadex_ido_primitives::FundingRoundWithPrimitives;
use sp_std::vec::Vec;
sp_api::decl_runtime_apis! {
    pub trait PolkadexIdoRuntimeApi<AccountId,Hash> where AccountId: Codec, Hash : Codec{
        fn rounds_by_investor(account : AccountId) -> Vec<(Hash, FundingRoundWithPrimitives<AccountId>)>;
        fn rounds_by_creator(account : AccountId) -> Vec<(Hash, FundingRoundWithPrimitives<AccountId>)> ;
    }
}
