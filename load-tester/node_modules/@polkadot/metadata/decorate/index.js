"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.expandMetadata = expandMetadata;
Object.defineProperty(exports, "decorateConstants", {
  enumerable: true,
  get: function () {
    return _constants.decorateConstants;
  }
});
Object.defineProperty(exports, "decorateExtrinsics", {
  enumerable: true,
  get: function () {
    return _extrinsics.decorateExtrinsics;
  }
});
Object.defineProperty(exports, "decorateStorage", {
  enumerable: true,
  get: function () {
    return _storage.decorateStorage;
  }
});

var _util = require("@polkadot/util");

var _Metadata = require("../Metadata");

var _constants = require("./constants");

var _errors = require("./errors");

var _events = require("./events");

var _extrinsics = require("./extrinsics");

var _storage = require("./storage");

// Copyright 2017-2021 @polkadot/metadata authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Expands the metadata by decoration into consts, query and tx sections
 */
function expandMetadata(registry, metadata) {
  (0, _util.assert)(metadata instanceof _Metadata.Metadata, 'You need to pass a valid Metadata instance to Decorated');
  const latest = metadata.asLatest;
  return {
    consts: (0, _constants.decorateConstants)(registry, latest),
    errors: (0, _errors.decorateErrors)(registry, latest, metadata.version),
    events: (0, _events.decorateEvents)(registry, latest, metadata.version),
    query: (0, _storage.decorateStorage)(registry, latest, metadata.version),
    tx: (0, _extrinsics.decorateExtrinsics)(registry, latest, metadata.version)
  };
}