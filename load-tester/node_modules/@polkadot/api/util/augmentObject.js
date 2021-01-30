"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.augmentObject = augmentObject;

var _util = require("@polkadot/util");

// Copyright 2017-2021 @polkadot/api authors & contributors
// SPDX-License-Identifier: Apache-2.0
const l = (0, _util.logger)('api/augment');

function logLength(type, values, and = []) {
  return values.length ? ` ${values.length} ${type}${and.length ? ' and' : ''}` : '';
}

function logValues(type, values) {
  return values.length ? `\n\t${type.padStart(7)}: ${values.sort().join(', ')}` : '';
} // log details to console


function warn(prefix, type, [added, removed]) {
  if (added.length || removed.length) {
    l.warn(`api.${prefix}: Found${logLength('added', added, removed)}${logLength('removed', removed)} ${type}:${logValues('added', added)}${logValues('removed', removed)}`);
  }
}

function extractKeys(src, dst) {
  return [Object.keys(src), Object.keys(dst)];
}

function findSectionExcludes(a, b) {
  return a.filter(section => !b.includes(section));
}

function extractSections(src, dst) {
  const [srcSections, dstSections] = extractKeys(src, dst);
  return [findSectionExcludes(srcSections, dstSections), findSectionExcludes(dstSections, srcSections)];
}

function findMethodExcludes(src, dst) {
  const srcSections = Object.keys(src);
  const dstSections = Object.keys(dst);
  return dstSections.filter(section => srcSections.includes(section)).reduce((rmMethods, section) => {
    const srcMethods = Object.keys(src[section]);
    return rmMethods.concat(...Object.keys(dst[section]).filter(method => !srcMethods.includes(method)).map(method => `${section}.${method}`));
  }, []);
}

function extractMethods(src, dst) {
  return [findMethodExcludes(dst, src), findMethodExcludes(src, dst)];
}
/**
 * Takes a decorated api section (e.g. api.tx) and augment it with the details. It does not override what is
 * already available, but rather just adds new missing ites into the result object.
 * @internal
 */


function augmentObject(prefix, src, dst, fromEmpty = false) {
  if (fromEmpty) {
    Object.keys(dst).forEach(key => {
      delete dst[key];
    });
  }

  if (prefix && Object.keys(dst).length) {
    warn(prefix, 'modules', extractSections(src, dst));
    warn(prefix, 'calls', extractMethods(src, dst));
  }

  return Object.keys(src).reduce((newSection, sectionName) => {
    const section = src[sectionName];
    newSection[sectionName] = Object.keys(section).reduce((result, methodName) => {
      // TODO When it does match, check the actual details and warn when there are differences
      if (!result[methodName]) {
        result[methodName] = section[methodName];
      }

      return result;
    }, dst[sectionName] || {});
    return newSection;
  }, dst);
}