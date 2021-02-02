// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0

/** @internal */
export function approvalFlagsToBools(flags) {
  const bools = [];
  flags.forEach(flag => {
    const str = flag.toString(2); // read from lowest bit to highest

    for (const bit of str.split('').reverse()) {
      bools.push(!!parseInt(bit, 10));
    }
  }); // slice off trailing "false" values, as in substrate

  const lastApproval = bools.lastIndexOf(true);
  return lastApproval >= 0 ? bools.slice(0, lastApproval + 1) : [];
}