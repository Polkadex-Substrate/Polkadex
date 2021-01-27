// Copyright 2017-2021 @polkadot/api authors & contributors
// SPDX-License-Identifier: Apache-2.0
// Most generic typings for `api.derive.*.*`
// Exact typings for a particular section `api.derive.section.*`
// Exact typings for all sections `api.derive.*.*`
// A technically unsafe version of Object.keys(obj) that assumes that
// obj only has known properties of T
function keys(obj) {
  return Object.keys(obj);
}
/**
 * This is a methods decorator which keeps all type information.
 */


function decorateMethods(section, decorateMethod) {
  return keys(section).reduce((acc, methodName) => {
    const method = section[methodName]; // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment

    acc[methodName] = decorateMethod(method);
    return acc;
  }, {});
}
/**
 * This is a section decorator which keeps all type information.
 */


export function decorateSections(allSections, decorateMethod) {
  return keys(allSections).reduce((acc, sectionName) => {
    acc[sectionName] = decorateMethods(allSections[sectionName], decorateMethod);
    return acc;
  }, {});
}