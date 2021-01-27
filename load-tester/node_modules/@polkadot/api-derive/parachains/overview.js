"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.overview = overview;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

var _util2 = require("./util");

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

function parse([ids, didUpdate, infos, pendingSwaps, relayDispatchQueueSizes]) {
  return ids.map((id, index) => ({
    didUpdate: (0, _util2.didUpdateToBool)(didUpdate, id),
    id,
    info: _objectSpread({
      id
    }, infos[index].unwrapOr(null)),
    pendingSwapId: pendingSwaps[index].unwrapOr(null),
    relayDispatchQueueSize: relayDispatchQueueSizes[index][0].toNumber()
  }));
}

function overview(instanceId, api) {
  return (0, _util.memo)(instanceId, () => {
    var _api$query$registrar;

    return (_api$query$registrar = api.query.registrar) !== null && _api$query$registrar !== void 0 && _api$query$registrar.parachains && api.query.parachains ? api.query.registrar.parachains().pipe((0, _operators.switchMap)(paraIds => (0, _xRxjs.combineLatest)([(0, _xRxjs.of)(paraIds), api.query.parachains.didUpdate(), api.query.registrar.paras.multi(paraIds), api.query.registrar.pendingSwap.multi(paraIds), api.query.parachains.relayDispatchQueueSize.multi(paraIds)])), (0, _operators.map)(parse)) : (0, _xRxjs.of)([]);
  });
}