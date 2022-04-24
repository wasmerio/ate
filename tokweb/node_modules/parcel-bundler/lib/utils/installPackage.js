'use strict';

function _asyncToGenerator(fn) { return function () { var gen = fn.apply(this, arguments); return new Promise(function (resolve, reject) { function step(key, arg) { try { var info = gen[key](arg); var value = info.value; } catch (error) { reject(error); return; } if (info.done) { resolve(value); } else { return Promise.resolve(value).then(function (value) { step("next", value); }, function (err) { step("throw", err); }); } } return step("next"); }); }; }

const spawn = require('cross-spawn');
const config = require('./config');
const path = require('path');

module.exports = (() => {
  var _ref = _asyncToGenerator(function* (dir, name) {
    let location = yield config.resolve(dir, ['yarn.lock', 'package.json']);

    return new Promise(function (resolve, reject) {
      let install;
      let options = {
        cwd: location ? path.dirname(location) : dir
      };

      if (location && path.basename(location) === 'yarn.lock') {
        install = spawn('yarn', ['add', name, '--dev'], options);
      } else {
        install = spawn('npm', ['install', name, '--save-dev'], options);
      }

      install.stdout.pipe(process.stdout);
      install.stderr.pipe(process.stderr);

      install.on('close', function (code) {
        if (code !== 0) {
          return reject(new Error(`Failed to install ${name}.`));
        }
        return resolve();
      });
    });
  });

  return function (_x, _x2) {
    return _ref.apply(this, arguments);
  };
})();