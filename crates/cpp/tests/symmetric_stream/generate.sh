#!/bin/sh
(cd stream/src ; ../../../../../../target/debug/wit-bindgen rust ../../wit/async_stream.wit --async none --symmetric)
(cd stream_cpp; ../../../../../target/debug/wit-bindgen cpp ../wit/async_stream.wit --symmetric)
cargo fmt
