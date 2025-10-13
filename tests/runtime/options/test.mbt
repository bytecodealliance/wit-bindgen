//@ [lang]
//@ path = 'gen/interface/test_/options/toTest/stub.mbt'

///|
pub fn option_none_param(a : String?) -> Unit {
  guard a is None
}

///|
pub fn option_some_param(a : String?) -> Unit {
  guard a is Some("foo")
}

///|
pub fn option_none_result() -> String? {
  None
}

///|
pub fn option_some_result() -> String? {
  Some("foo")
}

///|
pub fn option_roundtrip(a : String?) -> String? {
  a
}

///|
pub fn double_option_roundtrip(a : UInt??) -> UInt?? {
  a
}
