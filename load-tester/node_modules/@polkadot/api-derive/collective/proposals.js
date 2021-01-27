"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.proposals = proposals;
exports.proposal = proposal;

var _util = require("@polkadot/util");

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util2 = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function parse(api, [hashes, proposals, votes]) {
  return proposals.map((proposalOpt, index) => proposalOpt && proposalOpt.isSome ? {
    hash: api.registry.createType('Hash', hashes[index]),
    proposal: proposalOpt.unwrap(),
    votes: votes[index].unwrapOr(null)
  } : null).filter(proposal => !!proposal);
}

function _proposalsFrom(instanceId, api, section = 'council') {
  return (0, _util2.memo)(instanceId, hashes => {
    var _api$query$section;

    return ((0, _util.isFunction)((_api$query$section = api.query[section]) === null || _api$query$section === void 0 ? void 0 : _api$query$section.proposals) && hashes.length ? (0, _xRxjs.combineLatest)([(0, _xRxjs.of)(hashes), (0, _xRxjs.combineLatest)(hashes.map(hash => // this should simply be api.query[section].proposalOf.multi<Option<Proposal>>(hashes),
    // however we have had cases on Edgeware where the indices have moved around after an
    // upgrade, which results in invalid on-chain data
    api.query[section].proposalOf(hash).pipe((0, _operators.catchError)(() => (0, _xRxjs.of)(null))))), api.query[section].voting.multi(hashes)]) : (0, _xRxjs.of)([[], [], []])).pipe((0, _operators.map)(result => parse(api, result)));
  });
}

function proposals(instanceId, api, section = 'council') {
  const proposalsFrom = _proposalsFrom(instanceId, api, section);

  return (0, _util2.memo)(instanceId, () => {
    var _api$query$section2;

    return (0, _util.isFunction)((_api$query$section2 = api.query[section]) === null || _api$query$section2 === void 0 ? void 0 : _api$query$section2.proposals) ? api.query[section].proposals().pipe((0, _operators.switchMap)(proposalsFrom)) : (0, _xRxjs.of)([]);
  });
}

function proposal(instanceId, api, section = 'council') {
  const proposalsFrom = _proposalsFrom(instanceId, api, section);

  return (0, _util2.memo)(instanceId, hash => {
    var _api$query$section3;

    return (0, _util.isFunction)((_api$query$section3 = api.query[section]) === null || _api$query$section3 === void 0 ? void 0 : _api$query$section3.proposals) ? proposalsFrom([hash]).pipe((0, _operators.map)(([proposal]) => proposal)) : (0, _xRxjs.of)(null);
  });
}