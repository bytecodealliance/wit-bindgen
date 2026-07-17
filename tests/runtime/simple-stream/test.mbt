//@ [lang]
//@ path = 'gen/interface/my/test/i/stub.mbt'

pub async fn read_stream(
  x : @async-core.Stream[Unit],
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
      let _ = x.read(1) catch { _ => None }
      let _ = x.read(2) catch { _ => None }
      let _ = x.drop() catch { _ => () }
}
