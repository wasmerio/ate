'use strict';

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

const JSAsset = require('./JSAsset');

class JSONAsset extends JSAsset {
  load() {
    return _asyncToGenerator(function* () {
      return 'module.exports = ' + (yield super.load()) + ';';
    })();
  }

  parse() {}
  collectDependencies() {}
  pretransform() {}
  transform() {}
}

module.exports = JSONAsset;