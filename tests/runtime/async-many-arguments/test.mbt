//@ [lang]
//@ path = 'gen/interface/my/test/i/stub.mbt'

///|
pub async fn many_arguments(
  a : String,
  b : String,
  c : String,
  d : String,
  e : String,
  f : String,
  g : String,
  h : String,
  i : String,
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  assert_eq([a, b, c, d, e, f, g, h, i], [
    "a", "b", "c", "d", "e", "f", "g", "h", "i",
  ])
}

///|
pub async fn many_arguments_pending(
  x : @async-core.Future[Unit],
  a : String,
  b : String,
  c : String,
  d : String,
  e : String,
  f : String,
  g : String,
  h : String,
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  assert_eq([a, b, c, d, e, f, g, h], ["a", "b", "c", "d", "e", "f", "g", "h"])
  x.get()
}

///|
pub async fn many_arguments_cases(
  a : UInt?,
  b : Result[UInt, String],
  c : String,
  d : String,
  e : String,
  f : String,
  g : String,
  h : String,
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  // The caller sends either `(Some(42), Err("error"))` or `(None, Ok(7))`.
  match a {
    Some(_) => {
      assert_eq(a, Some(42))
      assert_eq(b, Err("error"))
    }
    None => assert_eq(b, Ok(7))
  }
  assert_eq([c, d, e, f, g, h], ["c", "d", "e", "f", "g", "h"])
}

///|
pub async fn one_big_record(
  r : BigRecord,
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  assert_eq(r.f0, 0)
  assert_eq(r.f8, 8)
  assert_eq(r.f16, 16)
}

///|
pub async fn direct_values(
  a : String,
  b : String,
  c : String,
  d : String,
  e : String,
  f : String,
  g : String,
  h : UInt,
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  assert_eq([a, b, c, d, e, f, g], ["a", "b", "c", "d", "e", "f", "g"])
  assert_eq(h, 8U)
}

///|
extern "wasm" fn memory_size() -> Int =
  #|(func (result i32) memory.size)

///|
/// Memory size in pages. Without the leak it stops growing once the warm-up
/// calls are done.
pub fn heap_mark() -> UInt {
  memory_size().reinterpret_as_uint()
}

///|
pub fn supports_cancel() -> Bool {
  true
}
