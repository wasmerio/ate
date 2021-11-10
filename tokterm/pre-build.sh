#!/bin/bash -e

mkdir -p dist
rm -r -f dist/*

cp -f www/index.html dist
cp -f -r node_modules/xterm/css/* dist
cp -f -r node_modules/xterm/lib/xterm.js.map dist

mkdir -p dist/bin
cp -f -r wasm/* dist/bin
