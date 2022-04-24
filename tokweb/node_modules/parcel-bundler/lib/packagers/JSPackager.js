'use strict';

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

const fs = require('fs');

var _require = require('path');

const basename = _require.basename;

const Packager = require('./Packager');

const prelude = fs.readFileSync(__dirname + '/../builtins/prelude.js', 'utf8').trim();
const hmr = fs.readFileSync(__dirname + '/../builtins/hmr-runtime.js', 'utf8').trim();

class JSPackager extends Packager {
  start() {
    var _this = this;

    return _asyncToGenerator(function* () {
      _this.first = true;
      _this.dedupe = new Map();

      yield _this.dest.write(prelude + '({');
    })();
  }

  addAsset(asset) {
    var _this2 = this;

    return _asyncToGenerator(function* () {
      if (_this2.dedupe.has(asset.generated.js)) {
        return;
      }

      // Don't dedupe when HMR is turned on since it messes with the asset ids
      if (!_this2.options.hmr) {
        _this2.dedupe.set(asset.generated.js, asset.id);
      }

      let deps = {};
      var _iteratorNormalCompletion = true;
      var _didIteratorError = false;
      var _iteratorError = undefined;

      try {
        for (var _iterator = asset.dependencies.values()[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true) {
          let dep = _step.value;

          let mod = asset.depAssets.get(dep.name);

          // For dynamic dependencies, list the child bundles to load along with the module id
          if (dep.dynamic && _this2.bundle.childBundles.has(mod.parentBundle)) {
            let bundles = [basename(mod.parentBundle.name)];
            var _iteratorNormalCompletion2 = true;
            var _didIteratorError2 = false;
            var _iteratorError2 = undefined;

            try {
              for (var _iterator2 = mod.parentBundle.siblingBundles.values()[Symbol.iterator](), _step2; !(_iteratorNormalCompletion2 = (_step2 = _iterator2.next()).done); _iteratorNormalCompletion2 = true) {
                let child = _step2.value;

                if (!child.isEmpty) {
                  bundles.push(basename(child.name));
                }
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

            bundles.push(mod.id);
            deps[dep.name] = bundles;
          } else {
            deps[dep.name] = _this2.dedupe.get(mod.generated.js) || mod.id;
          }
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

      yield _this2.writeModule(asset.id, asset.generated.js, deps);
    })();
  }

  writeModule(id, code, deps = {}) {
    var _this3 = this;

    return _asyncToGenerator(function* () {
      let wrapped = _this3.first ? '' : ',';
      wrapped += id + ':[function(require,module,exports) {\n' + (code || '') + '\n},';
      wrapped += JSON.stringify(deps);
      wrapped += ']';

      _this3.first = false;
      yield _this3.dest.write(wrapped);
    })();
  }

  end() {
    var _this4 = this;

    return _asyncToGenerator(function* () {
      let entry = [];

      // Add the HMR runtime if needed.
      if (_this4.options.hmr) {
        // Asset ids normally start at 1, so this should be safe.
        yield _this4.writeModule(0, hmr.replace('{{HMR_PORT}}', _this4.options.hmrPort));
        entry.push(0);
      }

      // Load the entry module
      if (_this4.bundle.entryAsset) {
        entry.push(_this4.bundle.entryAsset.id);
      }

      yield _this4.dest.end('},{},' + JSON.stringify(entry) + ')');
    })();
  }
}

module.exports = JSPackager;