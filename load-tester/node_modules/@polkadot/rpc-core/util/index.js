"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});

var _drr = require("./drr");

Object.keys(_drr).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _drr[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _drr[key];
    }
  });
});

var _refCountDelay = require("./refCountDelay");

Object.keys(_refCountDelay).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _refCountDelay[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _refCountDelay[key];
    }
  });
});