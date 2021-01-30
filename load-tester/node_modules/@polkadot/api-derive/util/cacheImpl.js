"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.deriveNoopCache = exports.deriveMapCache = void 0;
// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
const mapCache = new Map();
const deriveMapCache = {
  del: key => {
    mapCache.delete(key);
  },
  forEach: cb => {
    const entries = mapCache.entries();

    for (const entry in entries) {
      cb(entry[0], entry[1]);
    }
  },
  get: key => {
    return mapCache.get(key);
  },
  set: (key, value) => {
    mapCache.set(key, value);
  }
};
exports.deriveMapCache = deriveMapCache;
const deriveNoopCache = {
  del: () => undefined,
  forEach: () => undefined,
  get: () => undefined,
  set: (_, value) => value
};
exports.deriveNoopCache = deriveNoopCache;