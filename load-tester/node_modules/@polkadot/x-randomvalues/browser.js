"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.getRandomValues = getRandomValues;

// Copyright 2017-2021 @polkadot/x-randomvalues authors & contributors
// SPDX-License-Identifier: Apache-2.0
function getRandomValues(arr) {
  return crypto.getRandomValues(arr);
}