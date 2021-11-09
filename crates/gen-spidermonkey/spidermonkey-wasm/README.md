# `spidermonkey.wasm`

This directory contains the source code for `spidermonkey.wasm`, which is an
embedding of the SpiderMonkey JavaScript engine for targeting `wasm32-wasi` and
use with `wai-bindgen-gen-spidermonkey`. It exports a variety of helper
functions that are used by `wai-bindgen-gen-spidermonkey`'s generated glue
code. These helpers are typically named something like `SMW_whatever_function`.

## Building `spidermonkey.wasm`

```
make
```
