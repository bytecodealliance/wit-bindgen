#!/bin/sh
cd src
export TOPLEVEL=../../../../..
${TOPLEVEL}/target/debug/wit-bindgen rust ${TOPLEVEL}/tests/runtime-async/async/stream-string/test.wit --symmetric -w runner
cd ../test/src
export TOPLEVEL=../../../../../..
${TOPLEVEL}/target/debug/wit-bindgen rust ${TOPLEVEL}/tests/runtime-async/async/stream-string/test.wit --symmetric -w test
