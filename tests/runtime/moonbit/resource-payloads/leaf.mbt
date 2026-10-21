//@ [lang]
//@ path = 'gen/interface/my/test/leaf-interface/stub.mbt'
//@ pkg_config = '''
//@ import {
//@   "my/test/async-core" @async-core,
//@   "moonbitlang/core/encoding/utf8" @utf8,
//@ }
//@
//@ warnings = "-44"
//@
//@ supported_targets = "wasm"
//@ '''

///|
struct Body {
  mut contents : @async-core.Stream[Byte]?
  mut trailers : @async-core.Future[FieldsHandle]?
}

// This fixture requires collection before resource destruction. Its destructor
// assertions catch missed ownership transfers; they are not an abandonment
// policy for a production body. An API with async endpoint cleanup also needs
// an explicit async close operation before its synchronous resource drop.

///|
struct Response {
  mut body : BodyHandle?
}

///|
struct LeafThing(String)

///|
struct Fields(String)

///|
let leaf_drops : Ref[UInt] = { val: 0, }

///|
let leaf_live : Ref[UInt] = { val: 0, }

///|
let fields_live : Ref[UInt] = { val: 0, }

///|
let body_live : Ref[UInt] = { val: 0, }

///|
let response_live : Ref[UInt] = { val: 0, }

///|
pub fn LeafThing::leaf_thing(value : String) -> LeafThing {
  leaf_live.val += 1
  LeafThing(value)
}

///|
pub fn LeafThing::get(self : LeafThing) -> String {
  self.0
}

///|
fn LeafThing::dtor(_self : LeafThing) -> Unit {
  leaf_drops.val += 1
  leaf_live.val -= 1
}

///|
pub fn leaf_drop_count() -> UInt {
  leaf_drops.val
}

///|
pub fn leaf_live_count() -> UInt {
  leaf_live.val
}

///|
pub fn Fields::fields(value : String) -> Fields {
  fields_live.val += 1
  Fields(value)
}

///|
pub fn Fields::get(self : Fields) -> String {
  self.0
}

///|
fn Fields::dtor(_self : Fields) -> Unit {
  fields_live.val -= 1
}

///|
pub fn Body::body(
  contents : @async-core.Stream[Byte],
  trailers : @async-core.Future[FieldsHandle]?,
) -> Body {
  body_live.val += 1
  { contents: Some(contents), trailers, }
}

///|
pub async fn Body::collect(
  self : Body,
  _background_group : @async-core.TaskGroup[Unit],
) -> (String, String?) {
  guard! self.contents is Some(contents)
  self.contents = None
  let trailers = self.trailers
  self.trailers = None
  // Collection takes ownership of both endpoints before its first suspension.
  // They belong to this computation, so errors and cancellation must close them.
  defer @async-core.protect_from_cancel(() => {
    if trailers is Some(future) {
      future.drop()
    }
  })
  let bytes = {
    defer @async-core.protect_from_cancel(() => contents.drop())
    let bytes : Array[Byte] = []
    for ;; {
      match contents.read(16) {
        Some(chunk) =>
          for byte in chunk {
            bytes.push(byte)
          }
        None => break
      }
    }
    Bytes::from_array(bytes[:])
  }
  let trailers = match trailers {
    Some(future) => {
      let fields = future.get()
      defer fields.drop()
      Some(fields.get())
    }
    None => None
  }
  (@utf8.decode_lossy(bytes[:]), trailers)
}

///|
fn Body::dtor(self : Body) -> Unit {
  guard self.contents is None && self.trailers is None else {
    abort("test body dropped before collect")
  }
  body_live.val -= 1
}

///|
pub fn Response::response(body : BodyHandle) -> Response {
  response_live.val += 1
  { body: Some(body), }
}

///|
pub async fn Response::collect(
  self : Response,
  background_group : @async-core.TaskGroup[Unit],
) -> (String, String?) {
  guard! self.body is Some(body)
  self.body = None
  // The response hands ownership to the export's group. A forwarded method
  // may return before work in that group finishes; returning is not a join.
  // Register cleanup before calling it, including on error or cancellation.
  background_group.add_defer(() => body.drop())
  body.collect(background_group)
}

///|
fn Response::dtor(self : Response) -> Unit {
  guard self.body is None else { abort("test response dropped before collect") }
  response_live.val -= 1
}

///|
pub fn fields_live_count() -> UInt {
  fields_live.val
}

///|
pub fn body_live_count() -> UInt {
  body_live.val
}

///|
pub fn response_live_count() -> UInt {
  response_live.val
}

///|
suberror CollectionFailed {
  CollectionFailed
}

///|
pub async fn check_collection_cleanup(
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  let bodies = body_live.val
  let responses = response_live.val
  let fields = fields_live.val

  // A local producer failure propagates through both forwarding methods. A
  // local group gives this check an explicit point after all cleanup finishes.
  try
    @async-core.with_task_group(group => {
      let (contents, sink) = @async-core.Stream::new()
      sink.close()
      let trailers = @async-core.Future::from(async fn() {
        raise CollectionFailed
      })
      let response = ResponseHandle::response(
        BodyHandle::body(contents, Some(trailers)),
      )
      defer response.drop()
      ignore(response.collect(group))
    })
  catch {
    CollectionFailed => ()
    error => raise error
  } noraise {
    _ => abort("collection should propagate producer failure")
  }
  guard! body_live.val == bodies && response_live.val == responses

  // Cancel while collection is blocked in the contents stream, before it can
  // read the trailer. Both the endpoint and its unread resource must be freed.
  let (contents, sink) = @async-core.Stream::new()
  @async-core.with_task_group(group => {
    let trailers = @async-core.Future::ready_with_cleanup(
      FieldsHandle::fields("unread trailer"),
      fields => fields.drop(),
    )
    let response = ResponseHandle::response(
      BodyHandle::body(contents, Some(trailers)),
    )
    defer response.drop()
    let (started, start) = @async-core.Future::new()
    defer @async-core.protect_from_cancel(() => started.drop())
    let reader = group.spawn(
      async fn() {
        guard! start.complete(())
        ignore(response.collect(group))
      },
      allow_failure=true,
    )
    started.get()
    reader.cancel()
    try reader.wait() catch {
      @async-core.TaskCancelled::TaskCancelled => ()
      error => raise error
    } noraise {
      _ => abort("collection should be cancelled")
    }
  })
  guard! body_live.val == bodies && response_live.val == responses
  guard! fields_live.val == fields
  let unread : FixedArray[Byte] = [0]
  guard! !sink.write_all(unread[:])
  sink.close()
}
