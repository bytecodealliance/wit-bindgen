//@ [lang]
//@ path = 'gen/interface/test_/many_arguments/toTest/stub.mbt'
///|
pub fn many_arguments(
  _a1 : UInt64,
  _a2 : UInt64,
  _a3 : UInt64,
  _a4 : UInt64,
  _a5 : UInt64,
  _a6 : UInt64,
  _a7 : UInt64,
  _a8 : UInt64,
  _a9 : UInt64,
  _a10 : UInt64,
  _a11 : UInt64,
  _a12 : UInt64,
  _a13 : UInt64,
  _a14 : UInt64,
  _a15 : UInt64,
  _a16 : UInt64,
) -> Unit {
  try {
    assert_eq(_a1, 1)
    assert_eq(_a2, 2)
    assert_eq(_a3, 3)
    assert_eq(_a4, 4)
    assert_eq(_a5, 5)
    assert_eq(_a6, 6)
    assert_eq(_a7, 7)
    assert_eq(_a8, 8)
    assert_eq(_a9, 9)
    assert_eq(_a10, 10)
    assert_eq(_a11, 11)
    assert_eq(_a12, 12)
    assert_eq(_a13, 13)
    assert_eq(_a14, 14)
    assert_eq(_a15, 15)
    assert_eq(_a16, 16)
  } catch {
    _ => panic()
  }
}
