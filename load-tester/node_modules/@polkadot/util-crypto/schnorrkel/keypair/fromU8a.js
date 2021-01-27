"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.schnorrkelKeypairFromU8a = schnorrkelKeypairFromU8a;
// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
const SEC_LEN = 64;
const PUB_LEN = 32;

function schnorrkelKeypairFromU8a(full) {
  return {
    publicKey: full.slice(SEC_LEN, SEC_LEN + PUB_LEN),
    secretKey: full.slice(0, SEC_LEN)
  };
}