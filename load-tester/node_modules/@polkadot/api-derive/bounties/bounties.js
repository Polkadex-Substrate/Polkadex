"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.bounties = bounties;

var _util = require("@polkadot/api-derive/util");

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _filterBountyProposals = require("./helpers/filterBountyProposals");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function parseResult([maybeBounties, maybeDescriptions, ids, bountyProposals]) {
  const bounties = [];
  maybeBounties.forEach((bounty, index) => {
    if (bounty.isSome) {
      bounties.push({
        bounty: bounty.unwrap(),
        description: maybeDescriptions[index].unwrapOrDefault().toUtf8(),
        index: ids[index],
        proposals: bountyProposals.filter(bountyProposal => ids[index].eq(bountyProposal.proposal.args[0]))
      });
    }
  });
  return bounties;
}

function bounties(instanceId, api) {
  const bountyBase = api.query.bounties ? api.query.bounties : api.query.treasury;
  return (0, _util.memo)(instanceId, () => (0, _xRxjs.combineLatest)([bountyBase.bountyCount(), api.query.council ? api.query.council.proposalCount() : (0, _xRxjs.of)(0)]).pipe((0, _operators.switchMap)(() => (0, _xRxjs.combineLatest)([bountyBase.bounties.keys(), api.derive.council ? api.derive.council.proposals() : (0, _xRxjs.of)([])])), (0, _operators.switchMap)(([keys, proposals]) => {
    const ids = keys.map(({
      args: [id]
    }) => id);
    return (0, _xRxjs.combineLatest)([bountyBase.bounties.multi(ids), bountyBase.bountyDescriptions.multi(ids), (0, _xRxjs.of)(ids), (0, _xRxjs.of)((0, _filterBountyProposals.filterBountiesProposals)(api, proposals))]);
  }), (0, _operators.map)(parseResult)));
}