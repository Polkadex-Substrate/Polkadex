"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});

var _info = require("./info");

Object.keys(_info).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _info[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _info[key];
    }
  });
});

var _overview = require("./overview");

Object.keys(_overview).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _overview[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _overview[key];
    }
  });
});