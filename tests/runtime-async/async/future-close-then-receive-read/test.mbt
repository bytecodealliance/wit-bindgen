//@ [lang]
//@ path = 'gen/interface/a/b/theTest/stub.mbt'

///|
let slot : Ref[@async.CMFuture[Unit]?] = { val: None }

///|
pub async fn set(param : @async.CMFuture[Unit], task_group : @async.TaskGroup[Unit]) -> Unit {
  slot.val = Some(param)
}

///|
pub async fn get(task_group : @async.TaskGroup[Unit]) -> @async.CMFuture[Unit] {
  guard slot.val is Some(value)
  slot.val = None
  value
}
