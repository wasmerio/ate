'use strict';

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

const Path = require('path');
const crypto = require('crypto');

/**
 * A Bundle represents an output file, containing multiple assets. Bundles can have
 * child bundles, which are bundles that are loaded dynamically from this bundle.
 * Child bundles are also produced when importing an asset of a different type from
 * the bundle, e.g. importing a CSS file from JS.
 */
class Bundle {
  constructor(type, name, parent) {
    this.type = type;
    this.name = name;
    this.parentBundle = parent;
    this.entryAsset = null;
    this.assets = new Set();
    this.childBundles = new Set();
    this.siblingBundles = new Map();
  }

  addAsset(asset) {
    asset.bundles.add(this);
    this.assets.add(asset);
  }

  removeAsset(asset) {
    asset.bundles.delete(this);
    this.assets.delete(asset);
  }

  getSiblingBundle(type) {
    if (!type || type === this.type) {
      return this;
    }

    if (!this.siblingBundles.has(type)) {
      let bundle = this.createChildBundle(type, Path.join(Path.dirname(this.name), Path.basename(this.name, Path.extname(this.name)) + '.' + type));
      this.siblingBundles.set(type, bundle);
    }

    return this.siblingBundles.get(type);
  }

  createChildBundle(type, name) {
    let bundle = new Bundle(type, name, this);
    this.childBundles.add(bundle);
    return bundle;
  }

  get isEmpty() {
    return this.assets.size === 0;
  }

  package(bundler, oldHashes, newHashes = new Map()) {
    var _this = this;

    return _asyncToGenerator(function* () {
      if (_this.isEmpty) {
        return newHashes;
      }

      let hash = _this.getHash();
      newHashes.set(_this.name, hash);

      let promises = [];
      if (!oldHashes || oldHashes.get(_this.name) !== hash) {
        promises.push(_this._package(bundler));
      }

      var _iteratorNormalCompletion = true;
      var _didIteratorError = false;
      var _iteratorError = undefined;

      try {
        for (var _iterator = _this.childBundles.values()[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true) {
          let bundle = _step.value;

          promises.push(bundle.package(bundler, oldHashes, newHashes));
        }
      } catch (err) {
        _didIteratorError = true;
        _iteratorError = err;
      } finally {
        try {
          if (!_iteratorNormalCompletion && _iterator.return) {
            _iterator.return();
          }
        } finally {
          if (_didIteratorError) {
            throw _iteratorError;
          }
        }
      }

      yield Promise.all(promises);
      return newHashes;
    })();
  }

  _package(bundler) {
    var _this2 = this;

    return _asyncToGenerator(function* () {
      let Packager = bundler.packagers.get(_this2.type);
      let packager = new Packager(_this2, bundler);

      yield packager.start();

      let included = new Set();
      var _iteratorNormalCompletion2 = true;
      var _didIteratorError2 = false;
      var _iteratorError2 = undefined;

      try {
        for (var _iterator2 = _this2.assets[Symbol.iterator](), _step2; !(_iteratorNormalCompletion2 = (_step2 = _iterator2.next()).done); _iteratorNormalCompletion2 = true) {
          let asset = _step2.value;

          yield _this2._addDeps(asset, packager, included);
        }
      } catch (err) {
        _didIteratorError2 = true;
        _iteratorError2 = err;
      } finally {
        try {
          if (!_iteratorNormalCompletion2 && _iterator2.return) {
            _iterator2.return();
          }
        } finally {
          if (_didIteratorError2) {
            throw _iteratorError2;
          }
        }
      }

      yield packager.end();
    })();
  }

  _addDeps(asset, packager, included) {
    var _this3 = this;

    return _asyncToGenerator(function* () {
      if (!_this3.assets.has(asset) || included.has(asset)) {
        return;
      }

      included.add(asset);

      var _iteratorNormalCompletion3 = true;
      var _didIteratorError3 = false;
      var _iteratorError3 = undefined;

      try {
        for (var _iterator3 = asset.depAssets.values()[Symbol.iterator](), _step3; !(_iteratorNormalCompletion3 = (_step3 = _iterator3.next()).done); _iteratorNormalCompletion3 = true) {
          let depAsset = _step3.value;

          yield _this3._addDeps(depAsset, packager, included);
        }
      } catch (err) {
        _didIteratorError3 = true;
        _iteratorError3 = err;
      } finally {
        try {
          if (!_iteratorNormalCompletion3 && _iterator3.return) {
            _iterator3.return();
          }
        } finally {
          if (_didIteratorError3) {
            throw _iteratorError3;
          }
        }
      }

      yield packager.addAsset(asset);
    })();
  }

  getParents() {
    let parents = [];
    let bundle = this;

    while (bundle) {
      parents.push(bundle);
      bundle = bundle.parentBundle;
    }

    return parents;
  }

  findCommonAncestor(bundle) {
    // Get a list of parent bundles going up to the root
    let ourParents = this.getParents();
    let theirParents = bundle.getParents();

    // Start from the root bundle, and find the first bundle that's different
    let a = ourParents.pop();
    let b = theirParents.pop();
    let last;
    while (a === b && ourParents.length > 0 && theirParents.length > 0) {
      last = a;
      a = ourParents.pop();
      b = theirParents.pop();
    }

    if (a === b) {
      // One bundle descended from the other
      return a;
    }

    return last;
  }

  getHash() {
    let hash = crypto.createHash('md5');
    var _iteratorNormalCompletion4 = true;
    var _didIteratorError4 = false;
    var _iteratorError4 = undefined;

    try {
      for (var _iterator4 = this.assets[Symbol.iterator](), _step4; !(_iteratorNormalCompletion4 = (_step4 = _iterator4.next()).done); _iteratorNormalCompletion4 = true) {
        let asset = _step4.value;

        hash.update(asset.hash);
      }
    } catch (err) {
      _didIteratorError4 = true;
      _iteratorError4 = err;
    } finally {
      try {
        if (!_iteratorNormalCompletion4 && _iterator4.return) {
          _iterator4.return();
        }
      } finally {
        if (_didIteratorError4) {
          throw _iteratorError4;
        }
      }
    }

    return hash.digest('hex');
  }
}

module.exports = Bundle;