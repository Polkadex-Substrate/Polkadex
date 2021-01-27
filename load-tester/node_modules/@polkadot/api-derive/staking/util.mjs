// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
export function filterEras(eras, list) {
  return eras.filter(era => !list.some(entry => era.eq(entry.era)));
}