'use strict';

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

const CSSAsset = require('./CSSAsset');
const config = require('../utils/config');
const localRequire = require('../utils/localRequire');
const promisify = require('../utils/promisify');
const path = require('path');

class SASSAsset extends CSSAsset {
  parse(code) {
    var _this = this;

    return _asyncToGenerator(function* () {
      // node-sass should be installed locally in the module that's being required
      let sass = yield localRequire('node-sass', _this.name);
      let render = promisify(sass.render.bind(sass));

      let opts = _this.package.sass || (yield config.load(_this.name, ['.sassrc', '.sassrc.js'])) || {};
      opts.includePaths = (opts.includePaths || []).concat(path.dirname(_this.name));
      opts.data = code;
      opts.indentedSyntax = typeof opts.indentedSyntax === 'boolean' ? opts.indentedSyntax : path.extname(_this.name).toLowerCase() === '.sass';

      opts.functions = Object.assign({}, opts.functions, {
        url: function url(node) {
          let filename = _this.addURLDependency(node.getValue());
          return new sass.types.String(`url(${JSON.stringify(filename)})`);
        }
      });

      let res = yield render(opts);
      res.render = function () {
        return res.css.toString();
      };
      return res;
    })();
  }

  collectDependencies() {
    var _iteratorNormalCompletion = true;
    var _didIteratorError = false;
    var _iteratorError = undefined;

    try {
      for (var _iterator = this.ast.stats.includedFiles[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true) {
        let dep = _step.value;

        this.addDependency(dep, { includedInParent: true });
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
  }
}

module.exports = SASSAsset;