//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
let resolved : Ref[Bool] = { val: false }

///|
pub async fn pending_import(task_group : @async.TaskGroup[Unit]) -> Unit {
  resolved.val = false
  for {
    if resolved.val {
      return
    }
    @async.pause()
  }
}

///|
pub fn resolve_pending_import() -> Unit {
  resolved.val = true
}
