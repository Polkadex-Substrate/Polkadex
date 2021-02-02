"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.decorateStorage = decorateStorage;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

var _util = require("@polkadot/util");

var _createFunction = require("./createFunction");

var _getStorage = require("./getStorage");

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

/** @internal */
function decorateStorage(registry, {
  modules
}, metaVersion) {
  return modules.reduce((result, moduleMetadata) => {
    if (moduleMetadata.storage.isNone) {
      return result;
    }

    const {
      name
    } = moduleMetadata;
    const section = (0, _util.stringCamelCase)(name);
    const unwrapped = moduleMetadata.storage.unwrap();
    const prefix = unwrapped.prefix.toString(); // For access, we change the index names, i.e. System.Account -> system.account

    result[section] = unwrapped.items.reduce((newModule, meta) => {
      const method = meta.name.toString();
      newModule[(0, _util.stringLowerFirst)(method)] = (0, _createFunction.createFunction)(registry, {
        meta,
        method,
        prefix,
        section
      }, {
        metaVersion
      });
      return newModule;
    }, {});
    return result;
  }, _objectSpread({}, (0, _getStorage.getStorage)(registry, metaVersion)));
}