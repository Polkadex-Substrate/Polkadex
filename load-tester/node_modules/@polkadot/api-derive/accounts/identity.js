"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.identity = identity;
exports.hasIdentity = hasIdentity;
exports.hasIdentityMulti = hasIdentityMulti;

var _util = require("@polkadot/util");

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util2 = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
const UNDEF_HEX = {
  toHex: () => undefined
};

function dataAsString(data) {
  return data.isRaw ? (0, _util.u8aToString)(data.asRaw.toU8a(true)) : data.isNone ? undefined : data.toHex();
}

function extractOther(additional) {
  return additional.reduce((other, [_key, _value]) => {
    const key = dataAsString(_key);
    const value = dataAsString(_value);

    if (key && value) {
      other[key] = value;
    }

    return other;
  }, {});
}

function extractIdentity(identityOfOpt, superOf) {
  if (!(identityOfOpt !== null && identityOfOpt !== void 0 && identityOfOpt.isSome)) {
    return {
      judgements: []
    };
  }

  const {
    info,
    judgements
  } = identityOfOpt.unwrap();
  const topDisplay = dataAsString(info.display);
  return {
    display: superOf && dataAsString(superOf[1]) || topDisplay,
    displayParent: superOf && topDisplay,
    email: dataAsString(info.email),
    image: dataAsString(info.image),
    judgements,
    legal: dataAsString(info.legal),
    other: extractOther(info.additional),
    parent: superOf && superOf[0],
    pgp: info.pgpFingerprint.unwrapOr(UNDEF_HEX).toHex(),
    riot: dataAsString(info.riot),
    twitter: dataAsString(info.twitter),
    web: dataAsString(info.web)
  };
}

function getParent(api, identityOfOpt, superOfOpt) {
  if (identityOfOpt !== null && identityOfOpt !== void 0 && identityOfOpt.isSome) {
    // this identity has something set
    return (0, _xRxjs.of)([identityOfOpt, undefined]);
  } else if (superOfOpt !== null && superOfOpt !== void 0 && superOfOpt.isSome) {
    const superOf = superOfOpt.unwrap(); // we have a super

    return (0, _xRxjs.combineLatest)([api.query.identity.identityOf(superOf[0]), (0, _xRxjs.of)(superOf)]);
  } // nothing of value returned


  return (0, _xRxjs.of)([undefined, undefined]);
}

function getBase(api, accountId) {
  var _api$query$identity;

  return accountId && (_api$query$identity = api.query.identity) !== null && _api$query$identity !== void 0 && _api$query$identity.identityOf ? api.queryMulti([[api.query.identity.identityOf, accountId], [api.query.identity.superOf, accountId]]) : (0, _xRxjs.of)([undefined, undefined]);
}
/**
 * @name identity
 * @description Returns identity info for an account
 */


function identity(instanceId, api) {
  return (0, _util2.memo)(instanceId, accountId => getBase(api, accountId).pipe((0, _operators.switchMap)(([identityOfOpt, superOfOpt]) => getParent(api, identityOfOpt, superOfOpt)), (0, _operators.map)(([identityOfOpt, superOf]) => extractIdentity(identityOfOpt, superOf))));
}

function hasIdentity(instanceId, api) {
  return (0, _util2.memo)(instanceId, accountId => api.derive.accounts.hasIdentityMulti([accountId]).pipe((0, _operators.map)(([first]) => first)));
}

function hasIdentityMulti(instanceId, api) {
  return (0, _util2.memo)(instanceId, accountIds => {
    var _api$query$identity2;

    return (_api$query$identity2 = api.query.identity) !== null && _api$query$identity2 !== void 0 && _api$query$identity2.identityOf ? (0, _xRxjs.combineLatest)([api.query.identity.identityOf.multi(accountIds), api.query.identity.superOf.multi(accountIds)]).pipe((0, _operators.map)(([identities, supers]) => identities.map((identityOfOpt, index) => {
      const superOfOpt = supers[index];
      const parentId = superOfOpt && superOfOpt.isSome ? superOfOpt.unwrap()[0].toString() : undefined;
      let display;

      if (identityOfOpt && identityOfOpt.isSome) {
        const value = dataAsString(identityOfOpt.unwrap().info.display);

        if (value && !(0, _util.isHex)(value)) {
          display = value;
        }
      }

      return {
        display,
        hasIdentity: !!(display || parentId),
        parentId
      };
    }))) : (0, _xRxjs.of)(accountIds.map(() => ({
      hasIdentity: false
    })));
  });
}