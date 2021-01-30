"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.compareSet = compareSet;

var _util = require("@polkadot/util");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0
function compareSetArray(a, b) {
  // equal number of entries and each entry in the array should match
  return a.size === b.length && !b.some(entry => !a.has(entry));
} // NOTE These are used internally and when comparing objects, expects that
// when the second is an Set<string, Codec> that the first has to be as well


function compareSet(a, b) {
  if (Array.isArray(b)) {
    return compareSetArray(a, b);
  } else if (b instanceof Set) {
    return compareSetArray(a, [...b.values()]);
  } else if ((0, _util.isObject)(b)) {
    return compareSetArray(a, Object.values(b));
  }

  return false;
}