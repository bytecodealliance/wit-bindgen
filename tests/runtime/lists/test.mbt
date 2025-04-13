//@ [lang]
//@ path = 'gen/interface/test_/lists/toTest/stub.mbt'

///|
pub fn empty_list_param(a : FixedArray[Byte]) -> Unit {
  guard a is []

}

///|
pub fn empty_string_param(a : String) -> Unit {
  guard a is ""

}

///|
pub fn empty_list_result() -> FixedArray[Byte] {
  []
}

///|
pub fn empty_string_result() -> String {
  ""
}

///|
pub fn list_param(a : FixedArray[Byte]) -> Unit {
  guard a is [1, 2, 3, 4]

}

///|
pub fn list_param2(a : String) -> Unit {
  guard a is "foo"

}

///|
pub fn list_param3(a : Array[String]) -> Unit {
  guard a is ["foo", "bar", "baz"]

}

///|
pub fn list_param4(a : Array[Array[String]]) -> Unit {
  guard a is [["foo", "bar"], ["baz"]]

}

///|
pub fn list_param5(a : Array[(Byte, UInt, Byte)]) -> Unit {
  guard a is [(1, 2, 3), (4, 5, 6)]

}

///|
pub fn list_param_large(a : Array[String]) -> Unit {
  guard a.length() == 1000

}

///|
pub fn list_result() -> FixedArray[Byte] {
  [1, 2, 3, 4, 5]
}

///|
pub fn list_result2() -> String {
  "hello!"
}

///|
pub fn list_result3() -> Array[String] {
  ["hello,", "world!"]
}

///|
pub fn list_minmax8(
  a : FixedArray[Byte],
  b : Array[Int]
) -> (FixedArray[Byte], Array[Int]) {
  (a, b)
}

///|
pub fn list_minmax16(
  a : Array[UInt],
  b : Array[Int]
) -> (Array[UInt], Array[Int]) {
  (a, b)
}

///|
pub fn list_minmax32(
  a : FixedArray[UInt],
  b : FixedArray[Int]
) -> (FixedArray[UInt], FixedArray[Int]) {
  (a, b)
}

///|
pub fn list_minmax64(
  a : FixedArray[UInt64],
  b : FixedArray[Int64]
) -> (FixedArray[UInt64], FixedArray[Int64]) {
  (a, b)
}

///|
pub fn list_minmax_float(
  a : FixedArray[Float],
  b : FixedArray[Double]
) -> (FixedArray[Float], FixedArray[Double]) {
  (a, b)
}

///|
pub fn list_roundtrip(a : FixedArray[Byte]) -> FixedArray[Byte] {
  a
}

///|
pub fn string_roundtrip(a : String) -> String {
  a
}

///|
pub fn allocated_bytes() -> UInt {
  // not quite sure about this
  0
}
