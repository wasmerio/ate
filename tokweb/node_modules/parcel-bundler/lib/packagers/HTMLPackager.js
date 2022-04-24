'use strict';

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

const Packager = require('./Packager');
const posthtml = require('posthtml');
const path = require('path');
const urlJoin = require('../utils/urlJoin');

class HTMLPackager extends Packager {
  addAsset(asset) {
    var _this = this;

    return _asyncToGenerator(function* () {
      let html = asset.generated.html || '';

      // Find child bundles (e.g. JS) that have a sibling CSS bundle,
      // add them to the head so they are loaded immediately.
      let cssBundles = Array.from(_this.bundle.childBundles).map(function (b) {
        return b.siblingBundles.get('css');
      }).filter(Boolean);

      if (cssBundles.length > 0) {
        html = posthtml(_this.insertCSSBundles.bind(_this, cssBundles)).process(html, { sync: true }).html;
      }

      yield _this.dest.write(html);
    })();
  }

  insertCSSBundles(cssBundles, tree) {
    let head = find(tree, 'head');
    if (!head) {
      let html = find(tree, 'html');
      head = { tag: 'head' };
      html.content.unshift(head);
    }

    if (!head.content) {
      head.content = [];
    }

    var _iteratorNormalCompletion = true;
    var _didIteratorError = false;
    var _iteratorError = undefined;

    try {
      for (var _iterator = cssBundles[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true) {
        let bundle = _step.value;

        head.content.push({
          tag: 'link',
          attrs: {
            rel: 'stylesheet',
            href: urlJoin(this.options.publicURL, path.basename(bundle.name))
          }
        });
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

function find(tree, tag) {
  let res;
  tree.match({ tag }, node => {
    res = node;
    return node;
  });

  return res;
}

module.exports = HTMLPackager;