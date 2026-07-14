//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "import": ["test/list-in-variant/interface/test/list-in-variant/to-test"] }"""

///|
pub fn run() -> Unit {
  // list-in-option
  let r1 = @to-test.list_in_option(Some(["hello", "world"]))
  guard r1 == "hello,world"
  let r2 = @to-test.list_in_option(None)
  guard r2 == "none"

  // list-in-variant
  let r3 = @to-test.list_in_variant(@to-test.PayloadOrEmpty::WithData(["foo", "bar", "baz"]))
  guard r3 == "foo,bar,baz"
  let r4 = @to-test.list_in_variant(@to-test.PayloadOrEmpty::Empty)
  guard r4 == "empty"

  // list-in-result
  let r5 = @to-test.list_in_result(Ok(["a", "b", "c"]))
  guard r5 == "a,b,c"
  let r6 = @to-test.list_in_result(Err("oops"))
  guard r6 == "err:oops"

  // list-in-option-with-return (Bug 1 + Bug 2)
  let s1 = @to-test.list_in_option_with_return(Some(["hello", "world"]))
  guard s1.count == 2U
  guard s1.label == "hello,world"
  let s2 = @to-test.list_in_option_with_return(None)
  guard s2.count == 0U
  guard s2.label == "none"

  // top-level-list (contrast)
  let r7 = @to-test.top_level_list(["x", "y", "z"])
  guard r7 == "x,y,z"
}
