#!/bin/bash

set -ex

rm -rf static
mkdir static

cargo build -p witx-bindgen-demo --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/witx_bindgen_demo.wasm static/demo.wasm

cargo run js \
  --export crates/witx-bindgen-demo/browser.witx \
  --import crates/witx-bindgen-demo/demo.witx \
  --out-dir static

cp crates/witx-bindgen-demo/{index.html,main.ts} static/
(cd crates/witx-bindgen-demo && npx tsc ../../static/main.ts --target es6)

if [ ! -d ace ]; then
  mkdir ace
  cd ace
  curl -L https://github.com/ajaxorg/ace-builds/archive/refs/tags/v1.4.12.tar.gz | tar xzf -
  cd ..
fi

cp -r ace/ace-builds-1.4.12/src static/ace
