//@ [lang]
//@ path = 'gen/interface/test_/variants/toTest/stub.mbt'

///|
pub fn roundtrip_option(a : Float?) -> Byte? {
  a.map(fn(f) { f.to_int().to_byte() })
}

///|
pub fn roundtrip_result(a : Result[UInt, Float]) -> Result[Double, Byte] {
  match a {
    Ok(u) => Ok(u.to_double())
    Err(f) => Err(f.to_int().to_byte())
  }
}

///|
pub fn roundtrip_enum(a : E1) -> E1 {
  a
}

///|
pub fn invert_bool(a : Bool) -> Bool {
  not(a)
}

///|
pub fn variant_casts(a : (C1, C2, C3, C4, C5, C6)) -> (C1, C2, C3, C4, C5, C6) {
  a
}

///|
pub fn variant_zeros(a : (Z1, Z2, Z3, Z4)) -> (Z1, Z2, Z3, Z4) {
  a
}

///|
pub fn variant_typedefs(a : UInt?, b : Bool, c : Result[UInt, Unit]) -> Unit {
  guard a is None
  guard b is false
  guard c is Err(_)

}

///|
pub fn variant_enums(
  a : Bool,
  b : Result[Unit, Unit],
  c : MyErrno
) -> (Bool, Result[Unit, Unit], MyErrno) {
  guard a is true
  guard b is Ok(_)
  guard c is SUCCESS
  (a, b, c)
}
