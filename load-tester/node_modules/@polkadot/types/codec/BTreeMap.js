"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.BTreeMap = void 0;

var _Map = require("./Map");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0
class BTreeMap extends _Map.CodecMap {
  static with(keyType, valType) {
    return class extends BTreeMap {
      constructor(registry, value) {
        super(registry, keyType, valType, value, 'BTreeMap');
      }

    };
  }

}

exports.BTreeMap = BTreeMap;