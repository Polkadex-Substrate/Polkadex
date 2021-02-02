"use strict";

var _interopRequireWildcard = require("@babel/runtime/helpers/interopRequireWildcard");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.getStorage = getStorage;

var substrate = _interopRequireWildcard(require("./substrate"));

// Copyright 2017-2021 @polkadot/metadata authors & contributors
// SPDX-License-Identifier: Apache-2.0

/** @internal */
function getStorage(registry, metaVersion) {
  return {
    substrate: Object.entries(substrate).reduce((storage, [key, fn]) => {
      storage[key] = fn(registry, metaVersion);
      return storage;
    }, {})
  };
}