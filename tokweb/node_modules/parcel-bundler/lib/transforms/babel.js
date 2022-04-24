'use strict';

let shouldTransform = (() => {
  var _ref2 = _asyncToGenerator(function* (asset) {
    if (asset.isES6Module) {
      return true;
    }

    if (asset.ast) {
      return !!asset.babelConfig;
    }

    if (asset.package && asset.package.babel) {
      return true;
    }

    let babelrc = yield config.resolve(asset.name, ['.babelrc', '.babelrc.js']);
    return !!babelrc;
  });

  return function shouldTransform(_x2) {
    return _ref2.apply(this, arguments);
  };
})();

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

const babel = require('babel-core');
const config = require('../utils/config');

module.exports = (() => {
  var _ref = _asyncToGenerator(function* (asset) {
    if (!(yield shouldTransform(asset))) {
      return;
    }

    yield asset.parseIfNeeded();

    let config = {
      code: false,
      filename: asset.name
    };

    if (asset.isES6Module) {
      config.plugins = [require('babel-plugin-transform-es2015-modules-commonjs')];
    }

    let res = babel.transformFromAst(asset.ast, asset.contents, config);
    asset.ast = res.ast;
    asset.isAstDirty = true;
  });

  return function (_x) {
    return _ref.apply(this, arguments);
  };
})();