// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0
import { isFunction } from '@polkadot/util';
export function hasEq(o) {
  // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
  return isFunction(o.eq);
}