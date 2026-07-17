//@ wasmtime-flags = '-Wcomponent-model-async -S p3 -S http'
//@ [lang]
//@ path = 'gen/interface/wasi/http/handler/stub.mbt'
//@ pkg_config = """{ "import": [{ "path": "moonbitlang/core/buffer", "alias": "buffer" }, { "path": "example/background-hook/async-core", "alias": "async-core" }, { "path": "example/background-hook/interface/wasi/http/types", "alias": "types" }] }"""

///|
pub async fn handle(
  request : @types.Request,
  background_group : @async-core.TaskGroup[Unit],
) -> Result[@types.Response, @types.ErrorCode] {
  guard request.get_method() is @types.Method::Post else {
    request.drop()
    return Err(@types.ErrorCode::HttpRequestMethodInvalid)
  }
  guard request.get_path_with_query() == Some("/hooks/build") else {
    request.drop()
    return Err(@types.ErrorCode::HttpRequestUriInvalid)
  }

  let (handling_result, handling_promise) = @async-core.Future::new()
  let (request_body, request_trailers) = request.consume_body(handling_result)
  let payload = @buffer.Buffer()
  for ;; {
    match request_body.read(4096) {
      Some(chunk) => payload.write_iter(chunk.iter())
      None => break
    }
  }
  request_body.drop()
  match request_trailers.get() {
    Ok(Some(fields)) => fields.drop()
    Ok(None) => ()
    Err(error) => {
      let _ = handling_promise.complete(Err(error))
      return Err(error)
    }
  }
  let payload = payload.to_bytes()

  let (response, response_transmitted) = @types.Response::new(
    @types.Fields::fields(),
    None,
    @async-core.Future::ready(Ok(None)),
  )
  guard response.set_status_code(202U) is Ok(_) else {
    let error = @types.ErrorCode::InternalError(
      Some("failed to set hook response status"),
    )
    let _ = handling_promise.complete(Err(error))
    response.drop()
    return Err(error)
  }

  background_group.spawn_bg(async fn() {
    let result = (async fn() -> Result[Unit, @types.ErrorCode] {
      match response_transmitted.get() {
        Err(error) => return Err(error)
        Ok(_) => ()
      }
      let expected = b"build=release"
      guard payload.length() == expected.length() else {
        return Err(
          @types.ErrorCode::InternalError(Some("unexpected hook payload")),
        )
      }
      for i in 0..<expected.length() {
        guard payload[i] == expected[i] else {
          return Err(
            @types.ErrorCode::InternalError(Some("unexpected hook payload")),
          )
        }
      }
      Ok(())
    })() catch {
      _ => Err(
        @types.ErrorCode::InternalError(Some("background hook task failed")),
      )
    }
    let _ = handling_promise.complete(result)
  })
  Ok(response)
}
