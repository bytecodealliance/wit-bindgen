(component
  (import "a:b/the-test" (instance $test
    (export "x" (func))
  ))

  (core module $m
    (import "a:b/the-test" "x" (func $x))

    (func (export "run")
      call $x)
  )
  (core func $x (canon lower (func $test "x")))
  (core instance $i (instantiate $m
    (with "a:b/the-test" (instance
      (export "x" (func $x))
    ))
  ))

  (func (export "run") (canon lift (core func $i "run")))
)
