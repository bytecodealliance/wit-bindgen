//@ [lang]
//@ path = 'gen/interface/test_/records/toTest/stub.mbt'

///|
pub fn multiple_results() -> (Byte, UInt) {
  (4, 5)
}

///|
pub fn swap_tuple(a : (Byte, UInt)) -> (UInt, Byte) {
  (a.1, a.0)
}

///|
pub fn roundtrip_flags1(a : F1) -> F1 {
  a
}

///|
pub fn roundtrip_flags2(a : F2) -> F2 {
  a
}

///|
pub fn roundtrip_flags3(
  a : Flag8,
  b : Flag16,
  c : Flag32,
) -> (Flag8, Flag16, Flag32) {
  (a, b, c)
}

///|
pub fn roundtrip_record1(a : R1) -> R1 {
  a
}

///|
pub fn tuple1(a : Byte) -> Byte {
  a
}
