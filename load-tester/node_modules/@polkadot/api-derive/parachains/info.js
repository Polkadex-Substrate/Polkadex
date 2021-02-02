"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.info = info;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

var _util2 = require("./util");

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

function parseActive(id, active) {
  const found = active.find(([paraId]) => paraId === id);

  if (found && found[1].isSome) {
    const [collatorId, retriable] = found[1].unwrap();
    return _objectSpread({
      collatorId
    }, retriable.isWithRetries ? {
      isRetriable: true,
      retries: retriable.asWithRetries.toNumber()
    } : {
      isRetriable: false,
      retries: 0
    });
  }

  return null;
}

function parseCollators(id, collatorQueue) {
  return collatorQueue.map(queue => {
    const found = queue.find(([paraId]) => paraId === id);
    return found ? found[1] : null;
  });
}

function parse(id, [active, retryQueue, selectedThreads, didUpdate, info, pendingSwap, heads, relayDispatchQueue]) {
  if (info.isNone) {
    return null;
  }

  return {
    active: parseActive(id, active),
    didUpdate: (0, _util2.didUpdateToBool)(didUpdate, id),
    heads,
    id,
    info: _objectSpread({
      id
    }, info.unwrap()),
    pendingSwapId: pendingSwap.unwrapOr(null),
    relayDispatchQueue,
    retryCollators: parseCollators(id, retryQueue),
    selectedCollators: parseCollators(id, selectedThreads)
  };
}

function info(instanceId, api) {
  return (0, _util.memo)(instanceId, id => api.query.registrar && api.query.parachains ? api.queryMulti([api.query.registrar.active, api.query.registrar.retryQueue, api.query.registrar.selectedThreads, api.query.parachains.didUpdate, [api.query.registrar.paras, id], [api.query.registrar.pendingSwap, id], [api.query.parachains.heads, id], [api.query.parachains.relayDispatchQueue, id]]).pipe((0, _operators.map)(result => parse(api.registry.createType('ParaId', id), result))) : (0, _xRxjs.of)(null));
}