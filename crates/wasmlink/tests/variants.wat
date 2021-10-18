(module
    (memory (export "memory") 0)
    (func (export "canonical_abi_realloc") (param i32 i32 i32 i32) (result i32)
        unreachable
    )
    (func (export "canonical_abi_free") (param i32 i32 i32)
        unreachable
    )
    (func (export "e1-arg") (param i32)
        unreachable
    )
    (func (export "e1-result") (result i32)
        unreachable
    )
    (func (export "u1-arg") (param i32 i32)
        unreachable
    )
    (func (export "u1-result") (result i32)
        unreachable
    )
    (func (export "v1-arg") (param i32 i32 i32)
        unreachable
    )
    (func (export "v1-result") (result i32)
        unreachable
    )
    (func (export "bool-arg") (param i32)
        unreachable
    )
    (func (export "bool-result") (result i32)
        unreachable
    )
    (func (export "option-arg") (param i32 i32 i32 i32 i32 i32 i32 i32 f32 i32 i32 i32 i32 i32 i32)
        unreachable
    )
    (func (export "option-result") (result i32)
        unreachable
    )
    (func (export "casts") (param i32 i32 i32 f64 i32 i64 i32 i64 i32 i64 i32 i32 i32) (result i32)
        unreachable
    )
    (func (export "expected-arg") (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
        unreachable
    )
    (func (export "expected-result") (result i32)
        unreachable
    )
)
