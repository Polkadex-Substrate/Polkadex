"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.DoNotConstruct = void 0;

var _Null = require("./Null");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name DoNotConstruct
 * @description
 * An unknown type that fails on construction with the type info
 */
class DoNotConstruct extends _Null.Null {
  constructor(registry, typeName = 'DoNotConstruct') {
    super(registry);
    throw new Error(`Cannot construct unknown type ${typeName}`);
  }

  static with(typeName) {
    return class extends DoNotConstruct {
      constructor(registry) {
        super(registry, typeName);
      }

    };
  }

}

exports.DoNotConstruct = DoNotConstruct;