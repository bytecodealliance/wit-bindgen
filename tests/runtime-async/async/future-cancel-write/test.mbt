//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn take_then_drop(x : @async.FutureR[String]) -> Unit {
  x.drop()
}

///|
pub async fn read_and_drop(x : @async.FutureR[String]) -> Unit {
  let _ = x.get()

}
