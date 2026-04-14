//@ [lang]
//@ path = 'gen/interface/test_/maps/toTest/stub.mbt'

///|
pub fn named_roundtrip(a : Map[UInt, String]) -> Map[String, UInt] {
  let result : Map[String, UInt] = {}
  a.each(fn(id, name) { result[name] = id })
  result
}

///|
pub fn bytes_roundtrip(a : Map[String, FixedArray[Byte]]) -> Map[String, FixedArray[Byte]] {
  a
}

///|
pub fn empty_roundtrip(a : Map[UInt, String]) -> Map[UInt, String] {
  a
}

///|
pub fn option_roundtrip(a : Map[String, UInt?]) -> Map[String, UInt?] {
  a
}

///|
pub fn record_roundtrip(a : LabeledEntry) -> LabeledEntry {
  a
}

///|
pub fn inline_roundtrip(a : Map[UInt, String]) -> Map[String, UInt] {
  let result : Map[String, UInt] = {}
  a.each(fn(k, v) { result[v] = k })
  result
}

///|
pub fn large_roundtrip(a : Map[UInt, String]) -> Map[UInt, String] {
  a
}

///|
pub fn multi_param_roundtrip(a : Map[UInt, String], b : Map[String, FixedArray[Byte]]) -> (Map[String, UInt], Map[String, FixedArray[Byte]]) {
  let ids : Map[String, UInt] = {}
  a.each(fn(id, name) { ids[name] = id })
  (ids, b)
}

///|
pub fn nested_roundtrip(a : Map[String, Map[UInt, String]]) -> Map[String, Map[UInt, String]] {
  a
}

///|
pub fn variant_roundtrip(a : MapOrString) -> MapOrString {
  a
}

///|
pub fn result_roundtrip(a : Result[Map[UInt, String], String]) -> Result[Map[UInt, String], String] {
  a
}

///|
pub fn tuple_roundtrip(a : (Map[UInt, String], UInt64)) -> (Map[UInt, String], UInt64) {
  a
}

///|
pub fn single_entry_roundtrip(a : Map[UInt, String]) -> Map[UInt, String] {
  a
}
