//@ [lang]
//@ path = 'gen/interface/a/b/theTest/stub.mbt'

///|
pub async fn f(param : @async.Future[Byte], task_group : @async.TaskGroup[Unit]) -> Unit {
  assert_eq(param.get(), (0).to_byte())
}
