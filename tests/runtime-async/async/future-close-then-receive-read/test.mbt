//@ [lang]
//@ path = 'gen/interface/a/b/theTest/stub.mbt'

///|
let slot : Ref[@async.Future[Unit]?] = { val: None }

///|
pub async fn set(param : @async.Future[Unit], task_group : @async.TaskGroup[Unit]) -> Unit {
  slot.val = Some(param)
}

///|
pub async fn get(task_group : @async.TaskGroup[Unit]) -> @async.Future[Unit] {
  guard slot.val is Some(value)
  slot.val = None
  value
}
