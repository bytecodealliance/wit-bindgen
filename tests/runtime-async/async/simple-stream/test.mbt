//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

pub async fn read_stream(x : @async.StreamR[Unit], task_group : @async.TaskGroup[Unit]) -> Unit {
  guard (x.read)(1) is Some(_)
  guard (x.read)(2) is Some(_)
  (x.close)()
}
