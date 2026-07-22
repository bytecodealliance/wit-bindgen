# MoonBit Bindings Context

MoonBit emits core wasm and `wasm-tools` adapts it into a component. Generated
bindings expose WIT imports and adapt MoonBit exports to the component ABI.

Async design references:

- [Design contract](docs/async-design.md)
- [Terminology](docs/async-glossary.md)
- [FFI-boundary conversion decision](docs/adr/0001-async-ffi-boundary-conversion.md)
- [Local Future/Promise decision](docs/adr/0002-local-future-promise.md)

Lowercase `future` and `stream` refer to Component Model types. Uppercase
`Future` and `Stream` refer to local MoonBit types. Generated code converts
between them only at concrete WIT positions whose intrinsic names come from
`wit-parser`.
