//@ [lang]
//@ path = 'gen/interface/a/b/i/stub.mbt'

///|
pub async fn one_argument(x : UInt) -> Unit {
  assert_eq(x, 1)
}

///|
pub async fn one_result() -> UInt noraise {
  2
}

///|
pub async fn one_argument_and_result(x : UInt) -> UInt {
  assert_eq(x, 3)
  4
}

///|
pub async fn two_arguments(x : UInt, y : UInt) -> Unit {
  assert_eq(x, 5)
  assert_eq(y, 6)
}

///|
pub async fn two_arguments_and_result(x : UInt, y : UInt) -> UInt {
  assert_eq(x, 7)
  assert_eq(y, 8)
  9
}
