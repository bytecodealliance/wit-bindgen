(module
    (memory (export "memory") 0)
    (func (export "canonical_abi_realloc") (param i32 i32 i32 i32) (result i32)
        unreachable
    )
    (func (export "canonical_abi_free") (param i32 i32 i32)
        unreachable
    )
    (func (export "tuple_arg") (param i32 i32)
        unreachable
    )
    (func (export "tuple_result") (result i32)
        unreachable
    )
    (func (export "empty_arg")
        unreachable
    )
    (func (export "empty_result")
        unreachable
    )
    (func (export "scalar_arg") (param i32 i32)
        unreachable
    )
    (func (export "scalar_result") (result i32)
        unreachable
    )
    (func (export "flags_arg") (param i32)
        unreachable
    )
    (func (export "flags_result") (result i32)
        unreachable
    )
    (func (export "aggregate_arg") (param i32 i32 i32 i32 i32 i32)
        unreachable
    )
    (func (export "aggregate_result") (result i32)
        unreachable
    )
)
