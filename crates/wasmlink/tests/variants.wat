(module
    (memory (export "memory") 0)
    (func (export "canonical_abi_realloc") (param i32 i32 i32 i32) (result i32)
        unreachable
    )
    (func (export "canonical_abi_free") (param i32 i32 i32)
        unreachable
    )
    (func (export "e1_arg") (param i32)
        unreachable
    )
    (func (export "e1_result") (result i32)
        unreachable
    )
    (func (export "u1_arg") (param i32 i32)
        unreachable
    )
    (func (export "u1_result") (result i32)
        unreachable
    )
    (func (export "v1_arg") (param i32 i32 i32)
        unreachable
    )
    (func (export "v1_result") (result i32)
        unreachable
    )
    (func (export "bool_arg") (param i32)
        unreachable
    )
    (func (export "bool_result") (result i32)
        unreachable
    )
    (func (export "option_arg") (param i32 i32 i32 i32 i32 i32 i32 i32 f32 i32 i32 i32 i32 i32 i32)
        unreachable
    )
    (func (export "option_result") (result i32)
        unreachable
    )
    (func (export "casts") (param i32 i32 i32 f64 i32 i64 i32 i64 i32 i64 i32 i32 i32) (result i32)
        unreachable
    )
    (func (export "expected_arg") (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
        unreachable
    )
    (func (export "expected_result") (result i32)
        unreachable
    )
)
