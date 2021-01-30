"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
var _exportNames = {
  votingBalance: true,
  all: true
};
Object.defineProperty(exports, "all", {
  enumerable: true,
  get: function () {
    return _all.all;
  }
});
exports.votingBalance = void 0;

var _all = require("./all");

var _account = require("./account");

Object.keys(_account).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
  if (key in exports && exports[key] === _account[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _account[key];
    }
  });
});

var _fees = require("./fees");

Object.keys(_fees).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
  if (key in exports && exports[key] === _fees[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _fees[key];
    }
  });
});

var _votingBalances = require("./votingBalances");

Object.keys(_votingBalances).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
  if (key in exports && exports[key] === _votingBalances[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _votingBalances[key];
    }
  });
});
// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
const votingBalance = _all.all;
exports.votingBalance = votingBalance;