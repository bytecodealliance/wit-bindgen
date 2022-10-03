(module
  (import "new" "get-two" (func $get_two (param i32)))
  (import "env" "memory" (memory 0))

  (global $sp (mut i32) i32.const 0)

  (func (export "get_sum") (result i32)
    (local i32 i32)
    global.get $sp
    local.tee 0
    i32.const 8
    i32.sub
    local.tee 1
    global.set $sp

    local.get 1
    call $get_two

    (i32.add
      (i32.load (local.get 1))
      (i32.load offset=4 (local.get 1)))

    local.get 0
    global.set $sp
  )

)
