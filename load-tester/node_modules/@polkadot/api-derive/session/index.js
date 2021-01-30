"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});

var _eraLength = require("./eraLength");

Object.keys(_eraLength).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _eraLength[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _eraLength[key];
    }
  });
});

var _eraProgress = require("./eraProgress");

Object.keys(_eraProgress).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _eraProgress[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _eraProgress[key];
    }
  });
});

var _indexes = require("./indexes");

Object.keys(_indexes).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _indexes[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _indexes[key];
    }
  });
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

var _progress = require("./progress");

Object.keys(_progress).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _progress[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _progress[key];
    }
  });
});

var _sessionProgress = require("./sessionProgress");

Object.keys(_sessionProgress).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _sessionProgress[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _sessionProgress[key];
    }
  });
});