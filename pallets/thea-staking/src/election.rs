use polkadex_primitives::AccountId;
use sp_runtime::traits::Get;
use crate::{BalanceOf, Config};
use crate::session::Exposure;

/// Algorithm to elect relayers from all candidates
pub fn elect_relayers<T: Config>(mut candidates: Vec<(AccountId, Exposure<AccountId,BalanceOf<T>>)>) -> Vec<(AccountId, Exposure<AccountId,BalanceOf<T>>)>{
    // If we don't have preffered number of relayers we take everyone
    let max_relayers = T::MaxRelayers::get();
    if candidates.len() <= max_relayers as usize {
        return candidates
    }
    // Sort by the descending order of total stake if we have more candidates than MaxRelayers
    candidates.sort_unstable_by(| (_,ea),(_,eb) | {
        eb.total.cmp(&ea.total)
    });
    let _ = candidates.split_off(max_relayers as usize);
    // Take the top MaxRelayers relayers
    candidates
}