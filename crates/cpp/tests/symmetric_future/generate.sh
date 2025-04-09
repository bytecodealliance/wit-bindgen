#!/bin/sh
(cd future/src ; ../../../../../../target/debug/wit-bindgen rust ../../wit/future.wit --async none --symmetric)
(cd future_cpp; ../../../../../target/debug/wit-bindgen cpp ../wit/future.wit --symmetric)
cargo fmt
