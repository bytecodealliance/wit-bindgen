#!/bin/bash

set -ex

rm -rf static
mkdir static

cargo build -p wit-bindgen-demo --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/wit_bindgen_demo.wasm static/demo.wasm

cargo run js \
  --export crates/wit-bindgen-demo/browser.wit \
  --import crates/wit-bindgen-demo/demo.wit \
  --out-dir static

cp crates/wit-bindgen-demo/{index.html,main.ts} static/
(cd crates/wit-bindgen-demo && npx tsc ../../static/main.ts --target es6)

if [ ! -d ace ]; then
  mkdir ace
  cd ace
  curl -L https://github.com/ajaxorg/ace-builds/archive/refs/tags/v1.4.12.tar.gz | tar xzf -
  cd ..
fi

cp -r ace/ace-builds-1.4.12/src static/ace
