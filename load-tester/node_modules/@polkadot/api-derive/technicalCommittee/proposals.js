"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.proposals = proposals;

var _collective = require("../collective");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function proposals(instanceId, api) {
  return (0, _util.memo)(instanceId, (0, _collective.proposals)(instanceId, api, 'technicalCommittee'));
}