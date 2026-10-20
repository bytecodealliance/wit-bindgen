//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = '''
//@ import {
//@   "test/exported-resource-lifecycle/interface/test/exported-resource-lifecycle/api" @api,
//@ }
//@
//@ warnings = "-44"
//@
//@ supported_targets = "wasm"
//@ '''

///|
pub fn run() -> Unit {
  // Fallible constructors register only successful application objects.
  guard! @api.Tracked::tracked(0, false) is Err("zero")
  guard! @api.take_dropped() is []
  @api.Tracked::tracked(1, false).unwrap().drop()
  guard! @api.take_dropped() is [1]

  // Transferring ownership invalidates every alias to the outgoing wrapper.
  let value = @api.retain_alias(@api.Tracked::tracked(2, false).unwrap())
  @api.drop_saved_alias()
  guard! @api.take_dropped() is []
  value.drop()
  guard! @api.take_dropped() is [2]

  // Dropping a locally owned wrapper also invalidates its aliases.
  @api.drop_twice(@api.Tracked::tracked(3, false).unwrap())
  guard! @api.take_dropped() is [3]

  // Destruction may reuse its released slot. The child must remain usable
  // after the outer destructor finishes, with its own object still stored.
  @api.Tracked::tracked(4, true).unwrap().drop()
  guard! @api.take_dropped() is [4]
  @api.take_reentrant_child().drop()
  guard! @api.take_dropped() is [5]
}
