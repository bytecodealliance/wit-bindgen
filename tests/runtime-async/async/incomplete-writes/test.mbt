//@ [lang]
//@ path = 'gen/interface/my/test_/testInterface/stub.mbt'

///|
let test_values : Map[Int, String] = {}

///|
let next_test_rep : Ref[Int] = { val: 1 }

///| Destructor of the resource.
pub fn TestThing::dtor(self : TestThing) -> Unit {
  test_values.remove(self.rep())
}

///|
pub fn TestThing::test_thing(s : String) -> TestThing {
  let rep = next_test_rep.val
  next_test_rep.val += 1
  test_values[rep] = s
  TestThing::new(rep)
}

///|
pub fn TestThing::get(self : TestThing) -> String {
  test_values[self.rep()]
}

///|
pub async fn short_reads_test(
  s : @async.CMStream[TestThing],
  task_group : @async.TaskGroup[Unit],
) -> @async.CMStream[TestThing] {
  @async.CMStream::from(async fn(sink : @async.Sink[TestThing]) {
    let things : Array[TestThing] = []
    for {
      match s.read(1) {
        Some(chunk) => {
          for i = 0; i < chunk.length(); i = i + 1 {
            things.push(chunk[i])
          }
        }
        None => break
      }
    }
    s.close()
    if things.length() > 0 {
      let _ = sink.write(things[:])
    }
    sink.close()
  })
}

///|
pub async fn short_reads_leaf(
  s : @async.CMStream[@leafInterface.LeafThing],
  task_group : @async.TaskGroup[Unit],
) -> @async.CMStream[@leafInterface.LeafThing] {
  @async.CMStream::from(async fn(sink : @async.Sink[@leafInterface.LeafThing]) {
    let things : Array[@leafInterface.LeafThing] = []
    for {
      match s.read(1) {
        Some(chunk) => {
          for i = 0; i < chunk.length(); i = i + 1 {
            things.push(chunk[i])
          }
        }
        None => break
      }
    }
    s.close()
    if things.length() > 0 {
      let _ = sink.write(things[:])
    }
    sink.close()
  })
}

///|
pub async fn dropped_reader_test(
  f1 : @async.CMFuture[TestThing],
  f2 : @async.CMFuture[TestThing],
  task_group : @async.TaskGroup[Unit],
) -> (@async.CMFuture[TestThing], @async.CMFuture[TestThing]) {
  let (out1, writer1) : (@async.CMFuture[TestThing], @async.Promise[TestThing]) = @async.CMFuture::new()
  let (out2, writer2) : (@async.CMFuture[TestThing], @async.Promise[TestThing]) = @async.CMFuture::new()
  task_group.spawn_bg(async fn() {
    f1.drop()
    let thing = f2.get()
    writer1.close()
    writer2.write(thing)
  })
  (out1, out2)
}

///|
pub async fn dropped_reader_leaf(
  f1 : @async.CMFuture[@leafInterface.LeafThing],
  f2 : @async.CMFuture[@leafInterface.LeafThing],
  task_group : @async.TaskGroup[Unit],
) -> (@async.CMFuture[@leafInterface.LeafThing], @async.CMFuture[@leafInterface.LeafThing]) {
  let (out1, writer1) : (
    @async.CMFuture[@leafInterface.LeafThing],
    @async.Promise[@leafInterface.LeafThing],
  ) = @async.CMFuture::new()
  let (out2, writer2) : (
    @async.CMFuture[@leafInterface.LeafThing],
    @async.Promise[@leafInterface.LeafThing],
  ) = @async.CMFuture::new()
  task_group.spawn_bg(async fn() {
    f1.drop()
    let thing = f2.get()
    writer1.close()
    writer2.write(thing)
  })
  (out1, out2)
}
