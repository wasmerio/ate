'use strict';

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

const JSAsset = require('./JSAsset');
const config = require('../utils/config');
const localRequire = require('../utils/localRequire');

class TypeScriptAsset extends JSAsset {
  parse(code) {
    var _this = this;

    return _asyncToGenerator(function* () {
      // require typescript, installed locally in the app
      let typescript = yield localRequire('typescript', _this.name);
      let transpilerOptions = {
        compilerOptions: {
          module: typescript.ModuleKind.CommonJS,
          jsx: typescript.JsxEmit.Preserve
        },
        fileName: _this.basename
      };

      let tsconfig = yield config.load(_this.name, ['tsconfig.json']);

      // Overwrite default if config is found
      if (tsconfig) {
        transpilerOptions.compilerOptions = Object.assign(transpilerOptions.compilerOptions, tsconfig.compilerOptions);
      }
      transpilerOptions.compilerOptions.noEmit = false;

      // Transpile Module using TypeScript and parse result as ast format through babylon
      _this.contents = typescript.transpileModule(code, transpilerOptions).outputText;
      return yield super.parse(_this.contents);
    })();
  }
}

module.exports = TypeScriptAsset;