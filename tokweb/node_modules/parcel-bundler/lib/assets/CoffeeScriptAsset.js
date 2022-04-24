'use strict';

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

const JSAsset = require('./JSAsset');
const localRequire = require('../utils/localRequire');

class CoffeeScriptAsset extends JSAsset {
  parse(code) {
    var _this = this;

    return _asyncToGenerator(function* () {
      // require coffeescript, installed locally in the app
      let coffee = yield localRequire('coffeescript', _this.name);

      // Transpile Module using CoffeeScript and parse result as ast format through babylon
      _this.contents = coffee.compile(code, {});
      return yield super.parse(_this.contents);
    })();
  }
}

module.exports = CoffeeScriptAsset;