'use strict';

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

const fs = require('./utils/fs');
const path = require('path');
const md5 = require('./utils/md5');
const objectHash = require('./utils/objectHash');
const pkg = require('../package.json');
const json5 = require('json5');

// These keys can affect the output, so if they differ, the cache should not match
const OPTION_KEYS = ['publicURL', 'minify', 'hmr'];

class FSCache {
  constructor(options) {
    this.dir = path.resolve(options.cacheDir || '.cache');
    this.dirExists = false;
    this.invalidated = new Set();
    this.optionsHash = objectHash(OPTION_KEYS.reduce((p, k) => (p[k] = options[k], p), {
      version: pkg.version
    }));
  }

  ensureDirExists() {
    var _this = this;

    return _asyncToGenerator(function* () {
      yield fs.mkdirp(_this.dir);
      _this.dirExists = true;
    })();
  }

  getCacheFile(filename) {
    let hash = md5(this.optionsHash + filename);
    return path.join(this.dir, hash + '.json');
  }

  write(filename, data) {
    var _this2 = this;

    return _asyncToGenerator(function* () {
      try {
        yield _this2.ensureDirExists();
        yield fs.writeFile(_this2.getCacheFile(filename), JSON.stringify(data));
        _this2.invalidated.delete(filename);
      } catch (err) {
        console.error('Error writing to cache', err);
      }
    })();
  }

  read(filename) {
    var _this3 = this;

    return _asyncToGenerator(function* () {
      if (_this3.invalidated.has(filename)) {
        return null;
      }

      let cacheFile = _this3.getCacheFile(filename);

      try {
        let stats = yield fs.stat(filename);
        let cacheStats = yield fs.stat(cacheFile);

        if (stats.mtime > cacheStats.mtime) {
          return null;
        }

        let data = yield fs.readFile(cacheFile);
        return json5.parse(data);
      } catch (err) {
        return null;
      }
    })();
  }

  invalidate(filename) {
    this.invalidated.add(filename);
  }

  delete(filename) {
    var _this4 = this;

    return _asyncToGenerator(function* () {
      try {
        yield fs.unlink(_this4.getCacheFile(filename));
        _this4.invalidated.delete(filename);
      } catch (err) {
        // Fail silently
      }
    })();
  }
}

module.exports = FSCache;