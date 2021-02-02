"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});

require("./detectPackage");

var _codec = require("./codec");

Object.keys(_codec).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _codec[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _codec[key];
    }
  });
});

var _create = require("./create");

Object.keys(_create).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _create[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _create[key];
    }
  });
});

var _index = require("./index.types");

Object.keys(_index).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _index[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _index[key];
    }
  });
});