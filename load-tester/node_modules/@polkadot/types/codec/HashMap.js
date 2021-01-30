"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.HashMap = void 0;

var _Map = require("./Map");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0
class HashMap extends _Map.CodecMap {
  static with(keyType, valType) {
    return class extends HashMap {
      constructor(registry, value) {
        super(registry, keyType, valType, value);
      }

    };
  }

}

exports.HashMap = HashMap;