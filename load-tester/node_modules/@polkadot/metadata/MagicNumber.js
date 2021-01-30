"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.MagicNumber = exports.MAGIC_NUMBER = void 0;

var _primitive = require("@polkadot/types/primitive");

var _util = require("@polkadot/util");

// Copyright 2017-2021 @polkadot/metadata authors & contributors
// SPDX-License-Identifier: Apache-2.0
const MAGIC_NUMBER = 0x6174656d; // `meta`, reversed for Little Endian encoding

exports.MAGIC_NUMBER = MAGIC_NUMBER;

class MagicNumber extends _primitive.U32 {
  constructor(registry, value) {
    super(registry, value);

    if (!this.isEmpty) {
      const magic = registry.createType('u32', MAGIC_NUMBER);
      (0, _util.assert)(this.eq(magic), `MagicNumber mismatch: expected ${magic.toHex()}, found ${this.toHex()}`);
    }
  }

}

exports.MagicNumber = MagicNumber;