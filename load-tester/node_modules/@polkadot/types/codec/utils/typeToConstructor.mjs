// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0
import { isString } from '@polkadot/util';
export function typeToConstructor(registry, type) {
  return isString(type) ? registry.createClass(type) : type;
}