// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
export function filterBountiesProposals(api, allProposals) {
  const bountyTxBase = api.tx.bounties ? api.tx.bounties : api.tx.treasury;
  const bountyProposalCalls = [bountyTxBase.approveBounty, bountyTxBase.closeBounty, bountyTxBase.proposeCurator, bountyTxBase.unassignCurator];
  return allProposals.filter(proposal => bountyProposalCalls.find(bountyCall => bountyCall.is(proposal.proposal)));
}