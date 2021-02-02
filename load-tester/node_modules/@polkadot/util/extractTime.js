"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.extractTime = extractTime;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0
const HRS = 60 * 60;
const DAY = HRS * 24;
/**
 * @name addTime
 * @summary Add together two Time arrays
 */

function addTime(a, b) {
  return {
    days: a.days + b.days,
    hours: a.hours + b.hours,
    milliseconds: a.milliseconds + b.milliseconds,
    minutes: a.minutes + b.minutes,
    seconds: a.seconds + b.seconds
  };
}

const ZERO = {
  days: 0,
  hours: 0,
  milliseconds: 0,
  minutes: 0,
  seconds: 0
};

function extractDays(milliseconds, hrs) {
  const days = Math.floor(hrs / 24);
  return addTime(_objectSpread(_objectSpread({}, ZERO), {}, {
    days
  }), extractTime(milliseconds - days * DAY * 1000));
}

function extractHrs(milliseconds, mins) {
  const hrs = mins / 60;

  if (hrs < 24) {
    const hours = Math.floor(hrs);
    return addTime(_objectSpread(_objectSpread({}, ZERO), {}, {
      hours
    }), extractTime(milliseconds - hours * HRS * 1000));
  }

  return extractDays(milliseconds, hrs);
}

function extractMins(milliseconds, secs) {
  const mins = secs / 60;

  if (mins < 60) {
    const minutes = Math.floor(mins);
    return addTime(_objectSpread(_objectSpread({}, ZERO), {}, {
      minutes
    }), extractTime(milliseconds - minutes * 60 * 1000));
  }

  return extractHrs(milliseconds, mins);
}

function extractSecs(milliseconds) {
  const secs = milliseconds / 1000;

  if (secs < 60) {
    const seconds = Math.floor(secs);
    return addTime(_objectSpread(_objectSpread({}, ZERO), {}, {
      seconds
    }), extractTime(milliseconds - seconds * 1000));
  }

  return extractMins(milliseconds, secs);
}
/**
 * @name extractTime
 * @summary Convert a quantity of seconds to Time array representing accumulated {days, minutes, hours, seconds, milliseconds}
 * @example
 * <BR>
 *
 * ```javascript
 * import { extractTime } from '@polkadot/util';
 *
 * const { days, minutes, hours, seconds, milliseconds } = extractTime(6000); // 0, 0, 10, 0, 0
 * ```
 */


function extractTime(milliseconds) {
  if (!milliseconds) {
    return ZERO;
  } else if (milliseconds < 1000) {
    return _objectSpread(_objectSpread({}, ZERO), {}, {
      milliseconds
    });
  }

  return extractSecs(milliseconds);
}