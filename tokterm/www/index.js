import init, { start, resize } from '../dist/tokterm.js';
import 'regenerator-runtime/runtime.js'
import './workers-polyfill.js'

async function run() {
  Error.stackTraceLimit = 20;
  await init();
  await start();
}

run();
