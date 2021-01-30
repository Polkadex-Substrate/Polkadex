"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.preimage = preimage;

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

var _util2 = require("./util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function preimage(instanceId, api) {
  return (0, _util.memo)(instanceId, hash => api.query.democracy.preimages(hash).pipe((0, _operators.map)(imageOpt => (0, _util2.parseImage)(api, imageOpt))));
}