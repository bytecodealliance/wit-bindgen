//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn read_future(x : @async.FutureR[Unit]) -> Unit {
  x.get()
}

///|
pub async fn drop_future(x : @async.FutureR[Unit]) -> Unit {
  x.drop()
}
