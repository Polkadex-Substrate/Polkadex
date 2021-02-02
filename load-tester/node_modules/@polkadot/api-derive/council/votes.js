"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.votes = votes;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function retrieveStakeOf(api) {
  return (api.query.electionsPhragmen || api.query.elections).stakeOf.entries().pipe((0, _operators.map)(entries => entries.map(([key, stake]) => [key.args[0], stake])));
}

function retrieveVoteOf(api) {
  return (api.query.electionsPhragmen || api.query.elections).votesOf.entries().pipe((0, _operators.map)(entries => entries.map(([key, votes]) => [key.args[0], votes])));
}

function retrievePrev(api) {
  return (0, _xRxjs.combineLatest)([retrieveStakeOf(api), retrieveVoteOf(api)]).pipe((0, _operators.map)(([stakes, votes]) => {
    const result = [];
    votes.forEach(([voter, votes]) => {
      result.push([voter, {
        stake: api.registry.createType('Balance'),
        votes
      }]);
    });
    stakes.forEach(([staker, stake]) => {
      const entry = result.find(([voter]) => voter.eq(staker));

      if (entry) {
        entry[1].stake = stake;
      } else {
        result.push([staker, {
          stake,
          votes: []
        }]);
      }
    });
    return result;
  }));
}

function retrieveCurrent(api) {
  const elections = api.query.electionsPhragmen || api.query.elections;
  return elections.voting.entries().pipe((0, _operators.map)(entries => entries.map(([key, [stake, votes]]) => [key.args[0], {
    stake,
    votes
  }])));
}

function votes(instanceId, api) {
  return (0, _util.memo)(instanceId, () => (api.query.electionsPhragmen || api.query.elections).stakeOf ? retrievePrev(api) : retrieveCurrent(api));
}