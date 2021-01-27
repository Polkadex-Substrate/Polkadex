"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});

var _extrinsic = require("./extrinsic");

Object.keys(_extrinsic).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _extrinsic[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _extrinsic[key];
    }
  });
});

var _generic = require("./generic");

Object.keys(_generic).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _generic[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _generic[key];
    }
  });
});

var _primitive = require("./primitive");

Object.keys(_primitive).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _primitive[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _primitive[key];
    }
  });
});