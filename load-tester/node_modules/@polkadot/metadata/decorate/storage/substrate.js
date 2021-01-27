"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.childStorageKeyPrefix = exports.changesTrieConfig = exports.extrinsicIndex = exports.heapPages = exports.code = void 0;

var _createFunction = require("./createFunction");

// Copyright 2017-2021 @polkadot/metadata authors & contributors
// SPDX-License-Identifier: Apache-2.0
// Small helper function to factorize code on this page.

/** @internal */
function createRuntimeFunction(method, key, {
  documentation,
  type
}) {
  return (registry, metaVersion) => (0, _createFunction.createFunction)(registry, {
    meta: {
      documentation: registry.createType('Vec<Text>', [documentation]),
      modifier: registry.createType('StorageEntryModifierLatest', 1),
      // required
      toJSON: () => key,
      type: registry.createType('StorageEntryTypeLatest', type, 0)
    },
    method,
    prefix: 'Substrate',
    section: 'substrate'
  }, {
    key,
    metaVersion,
    skipHashing: true
  });
}

const code = createRuntimeFunction('code', ':code', {
  documentation: 'Wasm code of the runtime.',
  type: 'Bytes'
});
exports.code = code;
const heapPages = createRuntimeFunction('heapPages', ':heappages', {
  documentation: 'Number of wasm linear memory pages required for execution of the runtime.',
  type: 'u64'
});
exports.heapPages = heapPages;
const extrinsicIndex = createRuntimeFunction('extrinsicIndex', ':extrinsic_index', {
  documentation: 'Current extrinsic index (u32) is stored under this key.',
  type: 'u32'
});
exports.extrinsicIndex = extrinsicIndex;
const changesTrieConfig = createRuntimeFunction('changesTrieConfig', ':changes_trie', {
  documentation: 'Changes trie configuration is stored under this key.',
  type: 'u32'
});
exports.changesTrieConfig = changesTrieConfig;
const childStorageKeyPrefix = createRuntimeFunction('childStorageKeyPrefix', ':child_storage:', {
  documentation: 'Prefix of child storage keys.',
  type: 'u32'
});
exports.childStorageKeyPrefix = childStorageKeyPrefix;