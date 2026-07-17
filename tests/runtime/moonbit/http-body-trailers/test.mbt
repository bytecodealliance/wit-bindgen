//@ [lang]
//@ path = 'gen/interface/wasi/http/handler/stub.mbt'
//@ pkg_config = """{
//@   "import": [
//@     { "path": "my/test/async-core", "alias": "async-core" },
//@     { "path": "my/test/interface/wasi/http/types", "alias": "types" }
//@   ]
//@ }"""

///|
let body_progress : Ref[UInt] = Ref(0)

///|
pub async fn handle(
  request : @types.Request,
  background_group : @async-core.TaskGroup[Unit],
) -> Result[@types.Response, @types.ErrorCode] {
  request.drop()
  body_progress.val = 0
  let transmitted_done = Ref(false)
  let (completed, completed_sink) = @async-core.Stream::new(capacity=2)

  let trailers = @async-core.Future::from(async fn() {
    for _ in 0..<2 {
      guard completed.read(1) is Some(signal) && signal.length() == 1 else {
        abort("response completion signal closed early")
      }
    }
    guard body_progress.val >= 3 && transmitted_done.val else {
      abort("response completion signal arrived too early")
    }
    Ok(Some(@types.Fields::fields()))
  })

  let body = @async-core.Stream::produce(async fn(sink) {
    body_progress.val = 1
    let _ = sink.write_all_bytes(b"hello "[:])
    body_progress.val = 2
    let _ = sink.write_all_bytes(b"http p3"[:])
    sink.close()
    body_progress.val = 3
    let signal : FixedArray[Unit] = [()]
    guard completed_sink.write_all(signal[:]) else {
      abort("response completion signal closed early")
    }
  })

  let (response, transmitted) = @types.Response::new(
    @types.Fields::fields(),
    Some(body),
    trailers,
  )
  background_group.spawn_bg(async fn() {
    match transmitted.get() {
      Ok(_) => {
        transmitted_done.val = true
        let signal : FixedArray[Unit] = [()]
        guard completed_sink.write_all(signal[:]) else {
          abort("response completion signal closed early")
        }
      }
      Err(_) => abort("response transmission failed")
    }
  })
  Ok(response)
}
