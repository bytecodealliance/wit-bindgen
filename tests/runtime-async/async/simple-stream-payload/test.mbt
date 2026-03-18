//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

pub async fn read_stream(x : @async.Stream[Byte], task_group : @async.TaskGroup[Unit]) -> Unit {
  guard x.read(1) is Some(a)
  guard a.length() == 1
  guard a[0] == (0).to_byte()

  guard x.read(2) is Some(b)
  guard b.length() == 2
  guard b[0] == (1).to_byte()
  guard b[1] == (2).to_byte()

  guard x.read(1) is Some(c)
  guard c.length() == 1
  guard c[0] == (3).to_byte()

  guard x.read(1) is Some(d)
  guard d.length() == 1
  guard d[0] == (4).to_byte()

  x.close()
}
