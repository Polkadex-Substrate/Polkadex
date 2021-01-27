"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.dispatchQueue = dispatchQueue;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

var _util = require("@polkadot/util");

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util2 = require("../util");

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

const DEMOCRACY_ID = (0, _util.stringToHex)('democrac');

function queryQueue(api) {
  return api.query.democracy.dispatchQueue().pipe((0, _operators.switchMap)(dispatches => (0, _xRxjs.combineLatest)([(0, _xRxjs.of)(dispatches), api.derive.democracy.preimages(dispatches.map(([, hash]) => hash))])), (0, _operators.map)(([dispatches, images]) => dispatches.map(([at, imageHash, index], dispatchIndex) => ({
    at,
    image: images[dispatchIndex],
    imageHash,
    index
  }))));
}

function schedulerEntries(api) {
  // We don't get entries, but rather we get the keys (triggered via finished referendums) and
  // the subscribe to those keys - this means we pickup when the schedulers actually executes
  // at a block, the entry for that block will become empty
  return api.derive.democracy.referendumsFinished().pipe((0, _operators.switchMap)(() => api.query.scheduler.agenda.keys()), (0, _operators.switchMap)(keys => {
    const blockNumbers = keys.map(key => key.args[0]);
    return (0, _xRxjs.combineLatest)([(0, _xRxjs.of)(blockNumbers), api.query.scheduler.agenda.multi(blockNumbers)]);
  }));
}

function queryScheduler(api) {
  return schedulerEntries(api).pipe((0, _operators.switchMap)(([blockNumbers, agendas]) => {
    const result = [];
    blockNumbers.forEach((at, index) => {
      agendas[index].filter(optScheduled => optScheduled.isSome).forEach(optScheduled => {
        const scheduled = optScheduled.unwrap();

        if (scheduled.maybeId.isSome) {
          const id = scheduled.maybeId.unwrap().toHex();

          if (id.startsWith(DEMOCRACY_ID)) {
            const [, index] = api.registry.createType('(u64, ReferendumIndex)', id);
            const imageHash = scheduled.call.args[0];
            result.push({
              at,
              imageHash,
              index
            });
          }
        }
      });
    });
    return (0, _xRxjs.combineLatest)([(0, _xRxjs.of)(result), api.derive.democracy.preimages(result.map(({
      imageHash
    }) => imageHash))]);
  }), (0, _operators.map)(([infos, images]) => infos.map((info, index) => _objectSpread(_objectSpread({}, info), {}, {
    image: images[index]
  }))));
}

function dispatchQueue(instanceId, api) {
  return (0, _util2.memo)(instanceId, () => {
    var _api$query$scheduler;

    return (0, _util.isFunction)((_api$query$scheduler = api.query.scheduler) === null || _api$query$scheduler === void 0 ? void 0 : _api$query$scheduler.agenda) ? queryScheduler(api) : api.query.democracy.dispatchQueue ? queryQueue(api) : (0, _xRxjs.of)([]);
  });
}