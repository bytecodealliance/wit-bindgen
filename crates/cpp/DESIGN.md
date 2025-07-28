# Type mapping

| Code | Environment |
| --- | --- |
| G-- | guest side |
| H-- | host side |
| -I- | guest-import (guest calls) |
| -E- | guest-export (host calls) |
| --A | argument |
| --R | result |
| --S | in struct |

| mode | |
| --- | --- |
| v | passed by value |
| t | owernership transferred |
| p | cabi_post_ cleans up |

| API | | | ABI | |
| --- | --- | --- | --- | --- |
| 🌓 | asymmetric | | 📘 | canonical |
| ⚖️ | symmetric | | 🪞 | symmetric |

| Code | mode | WIT Type | Rust type | C++ Type | Lower | Reason |
| --- | --- | --- | --- | --- | --- | --- |
| GIA | v | string | &str[^1] | string_view (17) | addr, len | |
| | | list | &[T] | std::span [^5] | addr, len | |
| | | tuple | (...) | std::tuple | 0, 1, ...| |
| | | tuple<string, list> | (&str, &[T]) | std::tuple<...> | a,l,a,l |
| | | record{string, list} | &T | T const& | a,l,a,l |
| | | large-struct (>16 args) | &T | T const& | &t |
| | | result<string,list> | Result<&str, &[]> | std::expected<string_view, span> | d,a,l |
| | | option\<string> | Option\<&str> | optional<string_view> const& | d,a,l|
| | | list\<resrc> | &[\&Resrc]? | vector<string_view> const& | a,l|
| GIR | t | string | String | wit::string[^2] | &(addr, len) [^7] | |
| | | list | Vec | wit::vector | &(a,l) |
| | | result<string,list> | Result<String, Vec> | std::expected<wit::string, wit::vector> | &(d,a,l) |
| GEA | t | string | String | 🌓 wit::string | addr, len |
| | | | | ⚖️ string_view | |
| | | result<string,list> | Result<String, Vec> | 🌓 std::expected<wit::string, wit::vector> | d,a,l |
| | | | | ⚖️ std::expected<string_view, wit::span> | |
| GER | p | string | String | wit::string (or std?) | 📘 -> &(a,l) cabi_post_N:P/I#F [^6] |
| | | | | | 🪞 &(a,l) |
| | | result<string,list> | Result<String, Vec> | std::expected<wit::string, wit::vector> | 📘 -> &(d,a,l) cabi_post |
| --S | ? | string | String | wit::string | addr, len |
| HIA | v | string | | string_view | a,l |
| HIR | t | string | | wit::string[^3] | &(a,l) |
| HEA | t | string | | 🌓 wit::string[^4] | a,l |
| | | | | ⚖️ string_view [^5] | |
| HER | p | string | | 🌓 wit::guest_owned<string_view> | 📘 -> &(a,l) |
| | | | | ⚖️ wit::string [^5] | 🪞 &(a,l) |

[^1]: The host never frees memory (is never passed ownership)!

[^2]: A wit::string is identical to the canonical representation, so it can be part of structures. On the guest a wit::string owns the memory and frees it after use.
On the host a wit::string can be constructed(=allocated) with an exec_env argument. Thus, without an exec_env a wit::string on the host is inaccessible.
Complex (non-POD) struct elements on the host will need exec_env to decode or construct.

[^3]: A wit::string requires exec_env inside the host implementation. ~~Perhaps a flexible type (either std::string or wit::string would be possible), or make this a generation option?~~ std::string requires a copy, wit::string requires passing exec_env to the method (which is necessary for methods anyway).

[^4]: A host side wit::string doesn't own the data (not free in dtor), thus no move semantics.

[^5]: Not implemented, for now symmetric is priority

[^6]: Here the callee (guest) allocates the memory for the set on its side

[^7]: Caller passes address of the return object as argument

## [Symmetric ABI](https://github.com/WebAssembly/component-model/issues/386)

The idea is to directly connect (link) components to each other.

Thus imported and exported functions and resources need to be compatible
at the ABI level.

For now for functions the guest import convention is used in both directions:

- The imported function ABI is used with the following properties

  - (unchanged) List and string arguments are passed as Views, no free
    required, lifetime is constrained until the end of the call

  - (unchanged) Owned resources in arguments or results pass ownership
    to the callee

  - (unchanged) If there are too many (>1) flat results, a local
    uninitialized ret_area is passed via the last argument

  - (unchanged) Returned objects are owned.
    For functional safety, i.e. avoiding all
    allocations in the hot path, the hope is with [#385](https://github.com/WebAssembly/component-model/issues/385).

- The imported resource ABI is used also for exporting
  with one modification:

   Resource IDs become usize, so you can optimize the resource table away.

### Async functions (WASI 0.3)

- The *exported* ABI guides the symmetric choice, arguments are directly 
  passed like with synchronous function calls, if a return value is used a 
  return pointer is added. The returned value is either nullptr (finished)
  or the handle/address of an event subscription (see symmetric_executor crate).

  No need for externally visible callbacks as the code directly registered 
  its callbacks with the executor. No need for set_results.

- Stream and future handles become pointer size, the internal API is described via
  a WIT file (symmetric_stream)

## Structs proposal

See also the explanation of ownership at https://docs.rs/wit-bindgen/0.42.1/wit_bindgen/macro.generate.html

```
resource r; 
record d { s: string, l: list<r> }
arg: func(d: d);
result: func() -> d;
```

```
struct DResult {
  wit::string s;
  wit::list<R> l;
}
struct DParam {
  std::string_view s;
  std::span<R> l;
}
```

|direction|style|
|---|---|
|GIA|void arg(DParam d);|
|GIR|DResult result();|
|GEA|void arg(DResult d);|
|GER|DResult result();|
