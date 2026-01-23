//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

pub async fn read_stream(x : @async.StreamR[Unit], task_group : @async.TaskGroup[Unit]) -> Unit {
  while (x.read)(1) is Some(_) {

  }
}

