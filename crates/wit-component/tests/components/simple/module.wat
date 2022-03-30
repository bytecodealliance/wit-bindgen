(module
  (memory (export "memory") 1)
  (func (export "a") unreachable)
  (func (export "b") (result i32) unreachable)
  (func (export "c") (param i32 i32) (result i32) unreachable)
  (func (export "canonical_abi_realloc") (param i32 i32 i32 i32) (result i32) unreachable)
  (func (export "canonical_abi_free") (param i32 i32 i32) unreachable)
)