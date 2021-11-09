#!/bin/bash

set -ex

rm -rf static
mkdir static

cargo build -p wai-bindgen-demo --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/wai_bindgen_demo.wasm static/demo.wasm

cargo run js \
  --export crates/wai-bindgen-demo/browser.wai \
  --import crates/wai-bindgen-demo/demo.wai \
  --out-dir static

cp crates/wai-bindgen-demo/{index.html,main.ts} static/
(cd crates/wai-bindgen-demo && npx tsc ../../static/main.ts --target es6)

if [ ! -d ace ]; then
  mkdir ace
  cd ace
  curl -L https://github.com/ajaxorg/ace-builds/archive/refs/tags/v1.4.12.tar.gz | tar xzf -
  cd ..
fi

cp -r ace/ace-builds-1.4.12/src static/ace
