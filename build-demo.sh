set -ex

rm -rf static
mkdir static

cargo build -p demo --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/demo.wasm static/

cargo run -- js --import crates/demo/browser.witx --out-dir static/browser
cargo run -- js --export crates/demo/demo.witx --out-dir static/demo

cp crates/demo/index.html static/
cp crates/demo/main.ts static/
(cd crates/demo && npx tsc ../../static/main.ts --target es6)

if [ ! -d ace ]; then
  mkdir ace
  cd ace
  curl -L https://github.com/ajaxorg/ace-builds/archive/refs/tags/v1.4.12.tar.gz | tar xzf -
  cd ..
fi

cp -r ace/ace-builds-1.4.12/src static/ace
