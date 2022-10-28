#!/bin/bash

set -ex

rm -rf static
mkdir static

# Build the core wasm binary that will become a component
cargo build -p wit-bindgen-demo --target wasm32-unknown-unknown --release

# Translate the core wasm binary to a component
cargo run --release -p wit-component --bin wit-component -- \
  target/wasm32-unknown-unknown/release/wit_bindgen_demo.wasm -o target/demo.wasm

# Generate JS host bindings
cargo run host js target/demo.wasm --map "console=./console.js" --out-dir static

# Build JS from TypeScript and then copy in the ace editor as well.
cp crates/wit-bindgen-demo/{index.html,main.ts,console.js} static/
(cd crates/wit-bindgen-demo && npx tsc ../../static/main.ts --target es6)

if [ ! -d ace ]; then
  mkdir ace
  cd ace
  curl -L https://github.com/ajaxorg/ace-builds/archive/refs/tags/v1.4.12.tar.gz | tar xzf -
  cd ..
fi

cp -r ace/ace-builds-1.4.12/src static/ace
