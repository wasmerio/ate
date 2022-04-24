'use strict';

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

const fs = require('fs');
const promisify = require('../utils/promisify');

class Packager {
  constructor(bundle, bundler) {
    this.bundle = bundle;
    this.bundler = bundler;
    this.options = bundler.options;
    this.setup();
  }

  setup() {
    this.dest = fs.createWriteStream(this.bundle.name);
    this.dest.write = promisify(this.dest.write.bind(this.dest));
    this.dest.end = promisify(this.dest.end.bind(this.dest));
  }

  start() {
    return _asyncToGenerator(function* () {})();
  }

  // eslint-disable-next-line no-unused-vars
  addAsset(asset) {
    return _asyncToGenerator(function* () {
      throw new Error('Must be implemented by subclasses');
    })();
  }

  end() {
    var _this = this;

    return _asyncToGenerator(function* () {
      yield _this.dest.end();
    })();
  }
}

module.exports = Packager;