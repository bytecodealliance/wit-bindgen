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
| GIA | v | string | &str | string_view (17) | addr, len | |
| | | list | &[T] | span (20) | addr, len | |
| | | tuple | (...) | std::tuple | 0, 1, ...| |
| | | tuple<string, list> | (&str, &[T]) | std::tuple<...> | a,l,a,l | 
| | | record{string, list} | &T | T const& | a,l,a,l | 
| | | large-struct (>16 args) | &T | T const& | &t |
| | | result<string,list> | Result<&str, &[]> | std::expected<string_view, span> | d,a,l |
| GIR | t | string | String | wit::string | &(addr, len) | |
| | | list | Vec | wit::vector | &(a,l) |
| | | result<string,list> | Result<String, Vec> | std::expected<wit::string, wit::vector> | &(d,a,l) |
| GEA | t | string | String | wit::string&& | addr, len |
| | | result<string,list> | Result<String, Vec> | std::expected<wit::string, wit::vector>&& | d,a,l |
| GER | p | string | String | wit::string (or std?) | -> &(a,l) cabi_post_N:P/I#F |
| | | result<string,list> | Result<String, Vec> | std::expected<wit::string, wit::vector> | -> &(d,a,l) cabi_post |
| --S | ? | string | String | wit::string | addr, len |
| HIA | v | string | | string_view | a,l |
| HIR | t | string | | wit::string | &(a,l) |
| HEA | t | string | | wit::string&& | a,l | 
| HER | p | string | | string_view + special cleanup | -> &(a,l) |

Note: The host never frees memory (is never passed ownership)!
