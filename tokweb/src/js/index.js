import init, { start, resize } from "../../pkg/index.js";
import "regenerator-runtime/runtime.js";
import "./workers-polyfill.js";

async function run() {
  Error.stackTraceLimit = 20;
  await init();
  let terminal = document.getElementById("terminal");
  let frontBuffer = document.getElementById("frontBuffer");
  await start(terminal, frontBuffer);
}

run();
