"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.flattenUniq = flattenUniq;

// Copyright 2017-2021 @polkadot/metadata authors & contributors
// SPDX-License-Identifier: Apache-2.0

/** @internal */
function flattenUniq(list) {
  const flat = list.reduce((result, entry) => {
    return result.concat(Array.isArray(entry) ? flattenUniq(entry) : entry);
  }, []);
  return [...new Set(flat)].filter(value => value).sort();
}