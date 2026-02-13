//@ [lang]
//@ path = 'gen/interface/a/b/i/stub.mbt'

///|
pub async fn one_argument(x : String, task_group : @async.TaskGroup[Unit]) -> Unit {
  let copied = x + ""
  assert_eq(copied, "hello")
}
