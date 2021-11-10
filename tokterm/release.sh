#!/bin/bash -e
./pre-build.sh

wasm-pack build -t web

mv pkg/tokterm_bg.wasm pkg/tokterm_bg.big.wasm
wasm-opt --strip-debug --enable-reference-types -Oz -o pkg/tokterm_bg.wasm pkg/tokterm_bg.big.wasm
rm -f pkg/tokterm_bg.big.wasm

cp -f -r pkg/* dist

webpack
wait

rm -f dist/tokterm_bg.wasm
rm -f dist/tokterm_bg.wasm.d.ts
rm -f dist/tokterm.js
rm -f dist/tokterm.d.ts
