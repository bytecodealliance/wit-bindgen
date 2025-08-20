//@ [lang]
//@ path = 'gen/interface/test_/results/test/stub.mbt'

///|
pub fn string_error(_a : Float) -> Result[Float, String] {
  if _a == 0.0 {
    return Err("zero")
  }
  Ok(_a)
  
}

///|
pub fn enum_error(_a : Float) -> Result[Float, E] {
  if _a == 0.0 {
    return Err(E::A)
  }
  Ok(_a)
}

///|
pub fn record_error(_a : Float) -> Result[Float, E2] {
  if _a == 0.0 {
    return Err({ line: 420, column: 0 })
  } else if _a == 1.0 {
    return Err({ line: 77, column: 2 })
  }
  Ok(_a)
}

///|
pub fn variant_error(_a : Float) -> Result[Float, E3] {
  if _a == 0.0 {
    return Err(E3::E2({ line: 420, column: 0 }))
  } else if _a == 1.0 {
    return Err(E3::E1(B))
  } else if _a == 2.0 {
    return Err(E3::E1(C))
  }
  Ok(_a)
}

///|
pub fn empty_error(_a : UInt) -> Result[UInt, Unit] {
  if _a == 0 {
    Err(())
  } else if _a == 1 {
    Ok(42)
  } else {
    Ok(_a)
  }
}

///|
pub fn double_error(_a : UInt) -> Result[Result[Unit, String], String] {
  if _a == 0 {
    Ok(Ok(()))
  } else if _a == 1 {
    Ok(Err("one"))
  } else {
    Err("two")
  }
}
