'use strict';

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

const Asset = require('../Asset');
const parse = require('posthtml-parser');
const api = require('posthtml/lib/api');
const urlJoin = require('../utils/urlJoin');
const render = require('posthtml-render');
const posthtmlTransform = require('../transforms/posthtml');
const isURL = require('../utils/is-url');

// A list of all attributes that should produce a dependency
// Based on https://developer.mozilla.org/en-US/docs/Web/HTML/Attributes
const ATTRS = {
  src: ['script', 'img', 'audio', 'video', 'source', 'track', 'iframe', 'embed'],
  href: ['link', 'a'],
  poster: ['video']
};

class HTMLAsset extends Asset {
  constructor(name, pkg, options) {
    super(name, pkg, options);
    this.type = 'html';
    this.isAstDirty = false;
  }

  parse(code) {
    let res = parse(code);
    res.walk = api.walk;
    res.match = api.match;
    return res;
  }

  collectDependencies() {
    this.ast.walk(node => {
      if (node.attrs) {
        for (let attr in node.attrs) {
          let elements = ATTRS[attr];
          if (elements && elements.includes(node.tag)) {
            let assetPath = this.addURLDependency(node.attrs[attr]);
            if (!isURL(assetPath)) {
              assetPath = urlJoin(this.options.publicURL, assetPath);
            }
            node.attrs[attr] = assetPath;
            this.isAstDirty = true;
          }
        }
      }

      return node;
    });
  }

  transform() {
    var _this = this;

    return _asyncToGenerator(function* () {
      yield posthtmlTransform(_this);
    })();
  }

  generate() {
    let html = this.isAstDirty ? render(this.ast) : this.contents;
    return { html };
  }
}

module.exports = HTMLAsset;