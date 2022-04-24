'use strict';

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

var _require = require('uglify-es');

const minify = _require.minify;

const config = require('../utils/config');

module.exports = (() => {
  var _ref = _asyncToGenerator(function* (asset) {
    yield asset.parseIfNeeded();

    // Convert AST into JS
    let code = asset.generate().js;

    let customConfig = yield config.load(asset.name, ['.uglifyrc']);
    let options = {
      warnings: true,
      mangle: {
        toplevel: true
      },
      compress: {
        drop_console: true
      }
    };

    if (customConfig) {
      options = Object.assign(options, customConfig);
    }

    let result = minify(code, options);
    if (result.error) {
      throw result.error;
    }

    // Log all warnings
    if (result.warnings) {
      result.warnings.forEach(function (warning) {
        // TODO: warn this using the logger
        console.log(warning);
      });
    }

    // babel-generator did our code generation for us, so remove the old AST
    asset.ast = null;
    asset.outputCode = result.code;
    asset.isAstDirty = false;
  });

  return function (_x) {
    return _ref.apply(this, arguments);
  };
})();