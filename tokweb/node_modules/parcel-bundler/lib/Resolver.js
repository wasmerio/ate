'use strict';

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

const promisify = require('./utils/promisify');
const resolve = require('browser-resolve');
const resolveAsync = promisify(resolve);
const builtins = require('./builtins');
const path = require('path');
const glob = require('glob');

class Resolver {
  constructor(options = {}) {
    this.options = options;
    this.cache = new Map();
  }

  resolve(filename, parent) {
    var _this = this;

    return _asyncToGenerator(function* () {
      var resolved = yield _this.resolveInternal(filename, parent, resolveAsync);
      return _this.saveCache(filename, parent, resolved);
    })();
  }

  resolveSync(filename, parent) {
    var resolved = this.resolveInternal(filename, parent, resolve.sync);
    return this.saveCache(filename, parent, resolved);
  }

  resolveInternal(filename, parent, resolver) {
    let key = this.getCacheKey(filename, parent);
    if (this.cache.has(key)) {
      return this.cache.get(key);
    }

    if (glob.hasMagic(filename)) {
      return { path: path.resolve(path.dirname(parent), filename) };
    }

    let extensions = Object.keys(this.options.extensions);
    if (parent) {
      const parentExt = path.extname(parent);
      // parent's extension given high priority
      extensions = [parentExt, ...extensions.filter(ext => ext !== parentExt)];
    }

    return resolver(filename, {
      filename: parent,
      paths: this.options.paths,
      modules: builtins,
      extensions: extensions,
      packageFilter(pkg, pkgfile) {
        // Expose the path to the package.json file
        pkg.pkgfile = pkgfile;

        // libraries like d3.js specifies node.js specific files in the "main" which breaks the build
        // we use the "module" or "jsnext:main" field to get the full dependency tree if available
        const main = [pkg.module, pkg['jsnext:main']].find(entry => typeof entry === 'string');

        if (main) {
          pkg.main = main;
        }

        return pkg;
      }
    });
  }

  getCacheKey(filename, parent) {
    return (parent ? path.dirname(parent) : '') + ':' + filename;
  }

  saveCache(filename, parent, resolved) {
    if (Array.isArray(resolved)) {
      resolved = { path: resolved[0], pkg: resolved[1] };
    } else if (typeof resolved === 'string') {
      resolved = { path: resolved, pkg: null };
    }

    this.cache.set(this.getCacheKey(filename, parent), resolved);
    return resolved;
  }
}

module.exports = Resolver;