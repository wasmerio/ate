'use strict';

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

const WebSocket = require('ws');
const prettyError = require('./utils/prettyError');

class HMRServer {
  start() {
    var _this = this;

    return _asyncToGenerator(function* () {
      yield new Promise(function (resolve) {
        _this.wss = new WebSocket.Server({ port: 0 }, resolve);
      });

      _this.wss.on('connection', function (ws) {
        ws.onerror = _this.handleSocketError;
        if (_this.unresolvedError) {
          ws.send(JSON.stringify(_this.unresolvedError));
        }
      });

      _this.wss.on('error', _this.handleSocketError);

      return _this.wss._server.address().port;
    })();
  }

  stop() {
    this.wss.close();
  }

  emitError(err) {
    var _prettyError = prettyError(err);

    let message = _prettyError.message,
        stack = _prettyError.stack;

    // store the most recent error so we can notify new connections
    // and so we can broadcast when the error is resolved

    this.unresolvedError = {
      type: 'error',
      error: {
        message,
        stack
      }
    };

    this.broadcast(this.unresolvedError);
  }

  emitUpdate(assets) {
    if (this.unresolvedError) {
      this.unresolvedError = null;
      this.broadcast({
        type: 'error-resolved'
      });
    }

    const containsHtmlAsset = assets.some(asset => asset.type === 'html');
    if (containsHtmlAsset) {
      this.broadcast({
        type: 'reload'
      });
    } else {
      this.broadcast({
        type: 'update',
        assets: assets.map(asset => {
          let deps = {};
          var _iteratorNormalCompletion = true;
          var _didIteratorError = false;
          var _iteratorError = undefined;

          try {
            for (var _iterator = asset.dependencies.values()[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true) {
              let dep = _step.value;

              let mod = asset.depAssets.get(dep.name);
              deps[dep.name] = mod.id;
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

          return {
            id: asset.id,
            generated: asset.generated,
            deps: deps
          };
        })
      });
    }
  }

  handleSocketError(err) {
    if (err.code === 'ECONNRESET') {
      // This gets triggered on page refresh, ignore this
      return;
    }
    // TODO: Use logger to print errors
    console.log(prettyError(err));
  }

  broadcast(msg) {
    const json = JSON.stringify(msg);
    var _iteratorNormalCompletion2 = true;
    var _didIteratorError2 = false;
    var _iteratorError2 = undefined;

    try {
      for (var _iterator2 = this.wss.clients[Symbol.iterator](), _step2; !(_iteratorNormalCompletion2 = (_step2 = _iterator2.next()).done); _iteratorNormalCompletion2 = true) {
        let ws = _step2.value;

        ws.send(json);
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
  }
}

module.exports = HMRServer;