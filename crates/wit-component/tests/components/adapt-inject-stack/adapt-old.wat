(module
  (import "new" "get-two" (func $get_two (param i32)))
  (import "env" "memory" (memory 0))

  (global $__stack_pointer (mut i32) i32.const 0)
  (global $some_other_mutable_global (mut i32) i32.const 0)

  (func (export "get_sum") (result i32)
    (local i32 i32)
    global.get $__stack_pointer
    local.tee 0
    i32.const 8
    i32.sub
    local.tee 1
    global.set $__stack_pointer

    local.get 1
    call $get_two

    (i32.add
      (i32.load (local.get 1))
      (i32.load offset=4 (local.get 1)))

    (global.set $some_other_mutable_global (global.get $some_other_mutable_global))

    local.get 0
    global.set $__stack_pointer
  )

)
