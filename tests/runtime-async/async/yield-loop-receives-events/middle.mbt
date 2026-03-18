//@ [lang]
//@ path = 'gen/interface/test_/common/iRunner/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "import": ["test/common/interface/test_/common/iMiddle", "test/common/async"] }"""

///|
let hit : Ref[Bool] = { val: false }

///|
pub async fn f(task_group : @async.TaskGroup[Unit]) -> Unit {
  hit.val = false
  task_group.spawn_bg(async fn() {
    @iMiddle.f()
    hit.val = true
  })

  for ;; {
    if hit.val {
      return
    }
    @async.pause()
  }
}
