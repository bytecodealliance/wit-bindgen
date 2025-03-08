(component
  (core module $m
    (func (export "x"))
  )
  (core instance $i (instantiate $m))

  (func $x (canon lift (core func $i "x")))
  (instance $test (export "x" (func $x)))
  (export "a:b/the-test" (instance $test))
)
