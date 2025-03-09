#!/bin/sh
(cd future/src ; ../../../../../../target/debug/wit-bindgen rust ../../wit/future.wit --async none --symmetric)
cargo fmt
