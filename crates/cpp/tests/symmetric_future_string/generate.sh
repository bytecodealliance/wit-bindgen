#!/bin/sh

cd bindings
../../../../../target/debug/wit-bindgen rust ../test.wit --symmetric -w test
../../../../../target/debug/wit-bindgen rust ../test.wit --symmetric -w runner
