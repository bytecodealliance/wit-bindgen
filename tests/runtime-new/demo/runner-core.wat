(module
  (import "a:b/the-test" "x" (func $x))
  (memory (export "memory") 1)

  (func (export "_start")
    call $x
  )

)
