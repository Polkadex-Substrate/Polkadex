// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
import { u8aConcat } from '@polkadot/util';
export function schnorrkelKeypairToU8a({
  publicKey,
  secretKey
}) {
  return u8aConcat(secretKey, publicKey).slice();
}