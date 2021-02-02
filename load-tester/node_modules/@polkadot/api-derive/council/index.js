"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});

var _proposals = require("./proposals");

Object.keys(_proposals).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _proposals[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _proposals[key];
    }
  });
});

var _votes = require("./votes");

Object.keys(_votes).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _votes[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _votes[key];
    }
  });
});

var _votesOf = require("./votesOf");

Object.keys(_votesOf).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (key in exports && exports[key] === _votesOf[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _votesOf[key];
    }
  });
});