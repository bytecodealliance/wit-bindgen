//@ [lang]
//@ path = 'gen/interface/test_/flavorful/toTest/stub.mbt'

///|
pub fn f_list_in_record1(_a : ListInRecord1) -> Unit {
  try {
    assert_eq(_a.a, "list_in_record1")
  } catch {
    _ => panic()
  }
}

///|
pub fn f_list_in_record2() -> ListInRecord2 {
  { a: "list_in_record2" }
}

///|
pub fn f_list_in_record3(_a : ListInRecord3) -> ListInRecord3 {
  { a: "list_in_record3 output" }
}

///|
pub fn f_list_in_record4(_a : ListInRecord4) -> ListInRecord4 {
  { a: "result4".to_string() }
}

///|
pub fn f_list_in_variant1(
  _a : String?,
  _b : Result[Unit, String],
) -> Unit {
  try {
    assert_eq(_a.unwrap(), "foo")
    assert_eq(_b.unwrap_err(), "bar")
  } catch {
    _ => panic()
  }
}

///|
pub fn f_list_in_variant2() -> String? {
  Some("list_in_variant2".to_string())
}

///|
pub fn f_list_in_variant3(_a : String?) -> String? {
  try {
    assert_eq(_a.unwrap(), "input3")
  } catch {
    _ => panic()
  }
  Some("output3".to_string())
}

let first : Ref[Bool] = Ref::new(true);

///|
pub fn errno_result() -> Result[Unit, MyErrno] {
  if first.val {
    first.val = false;
    Err(MyErrno::B)
  } else {
    Ok(())
  }
}

pub fn write_utf8_char(buf : @buffer.T, value : Char) -> Unit {
  let code = value.to_uint()
  match code {
    _..<0x80 => {
      let b0 = ((code & 0x7F) | 0x00).to_byte()
      buf.write_byte(b0)
    }
    _..<0x0800 => {
      let b0 = (((code >> 6) & 0x1F) | 0xC0).to_byte()
      let b1 = ((code & 0x3F) | 0x80).to_byte()
      buf.write_byte(b0)
      buf.write_byte(b1)
    }
    _..<0x010000 => {
      let b0 = (((code >> 12) & 0x0F) | 0xE0).to_byte()
      let b1 = (((code >> 6) & 0x3F) | 0x80).to_byte()
      let b2 = ((code & 0x3F) | 0x80).to_byte()
      buf.write_byte(b0)
      buf.write_byte(b1)
      buf.write_byte(b2)
    }
    _..<0x110000 => {
      let b0 = (((code >> 18) & 0x07) | 0xF0).to_byte()
      let b1 = (((code >> 12) & 0x3F) | 0x80).to_byte()
      let b2 = (((code >> 6) & 0x3F) | 0x80).to_byte()
      let b3 = ((code & 0x3F) | 0x80).to_byte()
      buf.write_byte(b0)
      buf.write_byte(b1)
      buf.write_byte(b2)
      buf.write_byte(b3)
    }
    _ => abort("Char out of range")
  }
}

pub fn encode_to_utf8(
  src : String,
) -> Bytes{ 
  let buf = @buffer.new(size_hint=src.length() * 4)
  for c in src {
    write_utf8_char(buf, c);
  }
  buf.to_bytes()
}

///|
pub fn list_typedefs(
  _a : String,
  _c : Array[String],
) -> (FixedArray[Byte], Array[String]) {
  try {
    assert_eq(_a, "typedef1")
    assert_eq(_c.length(), 1)
    assert_eq(_c[0], "typedef2")
  } catch {
    _ => panic()
  }
  (encode_to_utf8("typedef3").to_fixedarray(), ["typedef4".to_string()])
}

///|
pub fn list_of_variants(
  _a : Array[Bool],
  _b : Array[Result[Unit, Unit]],
  _c : Array[MyErrno],
) -> (Array[Bool], Array[Result[Unit, Unit]], Array[MyErrno]) {
  try {
    assert_eq(_a, [true, false])
    assert_eq(_b, [Ok(()), Err(())])
    assert_eq(_c, [MyErrno::SUCCESS, MyErrno::A])
  } catch {
    _ => panic()
  }
  ([false, true], [Err(()), Ok(())], [MyErrno::A, MyErrno::B])
}
