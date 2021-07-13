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
    (func (export "acquire_an_x") (param i32 i32) (result i32)
        unreachable
    )
    (func (export "acquire_lots_of_x") (param i32 i32) (result i32)
        unreachable
    )
    (func (export "receive_an_x") (param i32) (result i32)
        unreachable
    )
    (func (export "receive_lots_of_x") (param i32 i32) (result i32)
        unreachable
    )
    (func (export "all_dropped") (result i32)
        unreachable
    )
)
