"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.isTestChain = isTestChain;
// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0
const re = /(Development|Local Testnet)$/;

function isTestChain(chain) {
  if (!chain) {
    return false;
  }

  return !!re.test(chain.toString());
}