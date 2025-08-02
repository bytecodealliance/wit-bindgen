//@ [lang]
//@ path = 'gen/interface/test_/strings/toTest/stub.mbt'

///|
pub fn take_basic(s : String) -> Unit {
  guard s == "latin utf16"

}

///|
pub fn return_unicode() -> String {
  "🚀🚀🚀 𠈄𓀀"
}

///|
pub fn return_empty() -> String {
  ""
}

///|
pub fn roundtrip(s : String) -> String {
  s
}
