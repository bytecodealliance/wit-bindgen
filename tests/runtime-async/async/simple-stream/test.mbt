//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

pub async fn read_stream(x : @async.CMStream[Unit], task_group : @async.TaskGroup[Unit]) -> Unit {
  guard x.read(1) is Some(a)
  guard a.length() == 1
  guard x.read(2) is Some(b)
  guard b.length() == 2
  x.close()
}
