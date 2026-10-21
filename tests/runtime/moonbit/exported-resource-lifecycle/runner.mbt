//@ wasmtime-flags = '-Wcomponent-model-async'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = '''
//@ import {
//@   "test/exported-resource-lifecycle/async-core" @async-core,
//@   "test/exported-resource-lifecycle/interface/test/exported-resource-lifecycle/api" @api,
//@ }
//@
//@ warnings = "-44"
//@
//@ supported_targets = "wasm"
//@ '''

///|
pub async fn run(_background_group : @async-core.TaskGroup[Unit]) -> Unit {
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

  // A local async handle method returns while the gate is closed, but its owned
  // argument stays alive until work in the supplied group finishes. Neither
  // early drop nor a join inside the forwarder can satisfy this ordering.
  let (gate, release) = @async-core.Future::new()
  let completion = @api.use_in_background(
    @api.Tracked::tracked(6, false).unwrap(),
    gate,
  )
  guard! @api.take_dropped() is []
  guard! release.complete(())
  completion.get()
  completion.drop()
  guard! @api.take_dropped() is [6]

  // Returned endpoint producers outlive the export's background group. The
  // producer owns its handle until it computes the value, even with no children.
  let result = @api.use_in_producer(@api.Tracked::tracked(7, false).unwrap())
  guard! @api.take_dropped() is []
  @api.release_producer()
  guard! result.get() == 7
  result.drop()
  guard! @api.take_dropped() is [7]

  // A lazy producer discarded before execution must release the same ownership.
  @api.discard_unstarted_producer(@api.Tracked::tracked(8, false).unwrap())
  guard! @api.take_dropped() is [8]

  // Reuse multiple vacant slots without disturbing their live neighbours.
  let first = @api.Tracked::tracked(9, false).unwrap()
  let second = @api.Tracked::tracked(10, false).unwrap()
  let third = @api.Tracked::tracked(11, false).unwrap()
  let fourth = @api.Tracked::tracked(12, false).unwrap()
  second.drop()
  fourth.drop()
  guard! @api.take_dropped() is [10, 12]
  let fifth = @api.Tracked::tracked(13, false).unwrap()
  let sixth = @api.Tracked::tracked(14, false).unwrap()
  guard! first.id() == 9 && third.id() == 11
  guard! fifth.id() == 13 && sixth.id() == 14
  sixth.drop()
  first.drop()
  fifth.drop()
  third.drop()
  guard! @api.take_dropped() is [14, 9, 13, 11]
}
