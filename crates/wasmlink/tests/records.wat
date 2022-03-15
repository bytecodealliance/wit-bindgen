(module
    (memory (export "memory") 0)
    (func (export "canonical_abi_realloc") (param i32 i32 i32 i32) (result i32)
        unreachable
    )
    (func (export "canonical_abi_free") (param i32 i32 i32)
        unreachable
    )
    (func (export "tuple-arg") (param i32 i32)
        unreachable
    )
    (func (export "tuple-result") (result i32)
        unreachable
    )
    (func (export "empty-arg")
        unreachable
    )
    (func (export "empty-result")
        unreachable
    )
    (func (export "scalar-arg") (param i32 i32)
        unreachable
    )
    (func (export "scalar-result") (result i32)
        unreachable
    )
    (func (export "flags-arg") (param i32)
        unreachable
    )
    (func (export "flags-result") (result i32)
        unreachable
    )
    (func (export "aggregate-arg") (param i32 i32 i32 i32 i32 i32)
        unreachable
    )
    (func (export "aggregate-result") (result i32)
        unreachable
    )
)
