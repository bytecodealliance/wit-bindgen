(module
    (memory (export "memory") 0)
    (func (export "canonical_abi_realloc") (param i32 i32 i32 i32) (result i32)
        unreachable
    )
    (func (export "canonical_abi_free") (param i32 i32 i32)
        unreachable
    )
    (func (export "canonical_abi_drop_x") (param i32)
        unreachable
    )
    (func (export "acquire-an-x") (param i32 i32) (result i32)
        unreachable
    )
    (func (export "acquire-lots-of-x") (param i32 i32) (result i32)
        unreachable
    )
    (func (export "receive-an-x") (param i32) (result i32)
        unreachable
    )
    (func (export "receive-lots-of-x") (param i32 i32) (result i32)
        unreachable
    )
    (func (export "all-dropped") (result i32)
        unreachable
    )
)
