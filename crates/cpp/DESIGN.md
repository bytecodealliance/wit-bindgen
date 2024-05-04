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

| Code | mode | WIT Type | Rust type | C++ Type | Lower | Reason |
| --- | --- | --- | --- | --- | --- | --- |
| GIA | v | string | &str[^1] | string_view (17) | addr, len | |
| | | list | &[T] | wit::span [^5] | addr, len | |
| | | tuple | (...) | std::tuple | 0, 1, ...| |
| | | tuple<string, list> | (&str, &[T]) | std::tuple<...> | a,l,a,l | 
| | | record{string, list} | &T | T const& | a,l,a,l | 
| | | large-struct (>16 args) | &T | T const& | &t |
| | | result<string,list> | Result<&str, &[]> | std::expected<string_view, span> | d,a,l |
| | | option\<string> | Option\<&str> | optional<string_view> const& | d,a,l|
| | | list\<resrc> | &[\&Resrc]? | vector<string_view> const& | a,l|
| GIR | t | string | String | wit::string[^2] | &(addr, len) | |
| | | list | Vec | wit::vector | &(a,l) |
| | | result<string,list> | Result<String, Vec> | std::expected<wit::string, wit::vector> | &(d,a,l) |
| GEA | t | string | String | wit::string&& | addr, len |
| | | result<string,list> | Result<String, Vec> | std::expected<wit::string, wit::vector>&& | d,a,l |
| GER | p | string | String | wit::string (or std?) | -> &(a,l) cabi_post_N:P/I#F |
| | | result<string,list> | Result<String, Vec> | std::expected<wit::string, wit::vector> | -> &(d,a,l) cabi_post |
| --S | ? | string | String | wit::string | addr, len |
| HIA | v | string | | string_view | a,l |
| HIR | t | string | | wit::string[^3] | &(a,l) |
| HEA | t | string | | wit::string[^4] | a,l | 
| HER | p | string | | wit::guest_owned<string_view> | -> &(a,l) |

[^1]: The host never frees memory (is never passed ownership)!

[^2]: A wit::string is identical to the canonical representation, so it can be part of structures. On the guest a wit::string owns the memory and frees it after use.
On the host a wit::string can be constructed(=allocated) with an exec_env argument. Thus, without an exec_env a wit::string on the host is inaccessible.
Complex (non-POD) struct elements on the host will need exec_env to decode or construct.

[^3]: A wit::string requires exec_env inside the host implementation. ~~Perhaps a flexible type (either std::string or wit::string would be possible), or make this a generation option?~~ std::string requires a copy, wit::string requires passing exec_env to the method (which is necessary for methods anyway).

[^4]: A host side wit::string doesn't own the data (not free in dtor), thus no move semantics.

[^5]: std::span requires C++-20, this alias should give minimal functionality with older compiler targets.
