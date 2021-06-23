# wasmtime

* Functions need to be able to at least optionally return a trap, e.g.
  `proc_raise` or they were passed an invalid buffer.

* buffer-in-buffer doesn't work. Doesn't work because we can't get a re-access
  of the transaction to add more buffers into it after-the-fact.

* Needs more testing on big-endian.

* Features from wiggle:
  * `async`
  * use `GuestError::InFunc` more liberally
    - stores/loads
    - `try_from` conversions
  * user-defined conversion from user-defined type to the actual type
    - used for converting `anyhow::Error` into either an errno or a trap
    - check for specific type of error, otherwise assume trap
    - conversion receives context to optionally store the error message
  * generate just the trait (??? what to do about `wasmtime` dep ???)

# JS

* Is there a better representation for general `variant` types? Currently it's
  `{ tag: string, val: T }` but that seems like it's probably sub-par. There's
  specializations for `option<T>` and `enum` variants, but that's it.

* Is there a better representation for flags than simply an integer?

* Should functions returning `expected<T, E>` get translated in JS to functions
  that return `T` and throw `E`?

* Adding imports to an import object is clunky because you need to also pass in
  a closure which extracts values from the raw instance. Unsure how to make this
  less clunky though.

* Needs more testing on big-endian. Specifically slice copies are probably not
  correct.

* Style with names needs to be respected, currently things are using
  `to_snake_case` but I think JS prefers camelCase?

* The `bigint` type is strict in that it does not accept plain `number` types to
  work with it. Should generated bindings be more flexible though and work with
  `number` in addition to `bigint`?

* Host-handle types are always ascribed as `any` but ideally we'd do better than
  that and assign them types. Maybe the type should be imported from somewhere
  else?

* Lifting/lowering of variants can almost surely use a more compressed technique
  which generates less code.

* Enums are handled in lowering as either strings or numbers, but should only
  numbers be handled here? Does anyone pass around strings as enum values?
