;;@ wasmtime-flags = '-Wcomponent-model-implements'

;; A `runner` written directly in WebAssembly text. It imports the same `store`
;; interface twice under two different plain names (`primary` and `backup`) via
;; the component model `implements` clause, so the two imports appear as distinct
;; core wasm import modules.
(module
  (import "primary" "set" (func $primary-set (param i32 i32 i32 i32)))
  (import "backup" "set" (func $backup-set (param i32 i32 i32 i32)))

  (memory (export "memory") 1)

  ;; "key" @ 0 (len 3), "primary" @ 3 (len 7), "backup" @ 10 (len 6)
  (data (i32.const 0) "keyprimarybackup")

  (func (export "run")
    ;; primary::set("key", "primary")
    (call $primary-set (i32.const 0) (i32.const 3) (i32.const 3) (i32.const 7))
    ;; backup::set("key", "backup")
    (call $backup-set (i32.const 0) (i32.const 3) (i32.const 10) (i32.const 6))
  )
)
