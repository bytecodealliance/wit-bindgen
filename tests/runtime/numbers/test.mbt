//@ [lang]
//@ path = 'gen/interface/test_/numbers/numbers/stub.mbt'

///|
pub fn roundtrip_u8(a : Byte) -> Byte {
  a
}

///|
pub fn roundtrip_s8(a : Int) -> Int {
  a
}

///|
pub fn roundtrip_u16(a : UInt) -> UInt {
  a
}

///|
pub fn roundtrip_s16(a : Int) -> Int {
  a
}

///|
pub fn roundtrip_u32(a : UInt) -> UInt {
  a
}

///|
pub fn roundtrip_s32(a : Int) -> Int {
  a
}

///|
pub fn roundtrip_u64(a : UInt64) -> UInt64 {
  a
}

///|
pub fn roundtrip_s64(a : Int64) -> Int64 {
  a
}

///|
pub fn roundtrip_f32(a : Float) -> Float {
  a
}

///|
pub fn roundtrip_f64(a : Double) -> Double {
  a
}

///|
pub fn roundtrip_char(a : Char) -> Char {
  a
}

///|
let global_ref : Ref[UInt] = { val: 0U }

///|
pub fn set_scalar(a : UInt) -> Unit {
  global_ref.val = a
}

///|
pub fn get_scalar() -> UInt {
  global_ref.val
}
