"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.memoize = memoize;

var _bigInt = require("./is/bigInt");

var _undefined = require("./is/undefined");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0
function defaultGetId() {
  return 'none';
}

function normalize(args) {
  return JSON.stringify(args, (_, value) => (0, _bigInt.isBigInt)(value) ? value.toString() : value);
} // eslint-disable-next-line @typescript-eslint/no-explicit-any


function memoize(fn, {
  getInstanceId = defaultGetId
} = {}) {
  const cache = {};

  const memoized = (...args) => {
    const stringParams = normalize(args);
    const instanceId = getInstanceId();

    if (!cache[instanceId]) {
      cache[instanceId] = {};
    }

    if ((0, _undefined.isUndefined)(cache[instanceId][stringParams])) {
      cache[instanceId][stringParams] = fn(...args);
    }

    return cache[instanceId][stringParams];
  };

  memoized.unmemoize = (...args) => {
    const stringParams = normalize(args);
    const instanceId = getInstanceId();

    if (cache[instanceId] && !(0, _undefined.isUndefined)(cache[instanceId][stringParams])) {
      delete cache[instanceId][stringParams];
    }
  };

  return memoized;
}