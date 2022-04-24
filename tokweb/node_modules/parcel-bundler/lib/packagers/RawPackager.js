'use strict';

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

const Packager = require('./Packager');
const fs = require('../utils/fs');
const path = require('path');
const url = require('url');

class RawPackager extends Packager {
  // Override so we don't create a file for this bundle.
  // Each asset will be emitted as a separate file instead.
  setup() {}

  addAsset(asset) {
    var _this = this;

    return _asyncToGenerator(function* () {
      // Use the bundle name if this is the entry asset, otherwise generate one.
      let name = _this.bundle.name;
      if (asset !== _this.bundle.entryAsset) {
        name = url.resolve(path.join(path.dirname(_this.bundle.name), asset.generateBundleName()), '');
      }

      let contents = asset.generated[asset.type] || (yield fs.readFile(asset.name));
      yield fs.writeFile(name, contents);
    })();
  }

  end() {}
}

module.exports = RawPackager;