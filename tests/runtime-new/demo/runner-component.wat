(component
  (import "a:b/the-test" (instance $test
    (export "x" (func))
  ))

  (core module $m
    (import "a:b/the-test" "x" (func $x))

    (func (export "run") (result i32)
      call $x
      i32.const 0)
  )
  (core func $x (canon lower (func $test "x")))
  (core instance $i (instantiate $m
    (with "a:b/the-test" (instance
      (export "x" (func $x))
    ))
  ))

  (func $run (result (result)) (canon lift (core func $i "run")))
  (instance $run (export "run" (func $run)))
  (export "wasi:cli/run@0.2.0" (instance $run))
)
