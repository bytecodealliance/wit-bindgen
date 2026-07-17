//@ wasmtime-flags = '-Wcomponent-model-async -S p3 -S http'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "import": [{ "path": "example/background-hook/async-core", "alias": "async-core" }, { "path": "example/background-hook/interface/wasi/http/handler", "alias": "handler" }, { "path": "example/background-hook/interface/wasi/http/types", "alias": "types" }] }"""

///|
pub async fn run(_background_group : @async-core.TaskGroup[Unit]) -> Unit {
  let request_body = @async-core.Stream::produce(async fn(sink) {
    guard sink.write_all_bytes(b"build=debug"[:]) else {
      abort("request body closed early")
    }
    sink.close()
  })
  let (request, request_transmitted) = @types.Request::new(
    @types.Fields::fields(),
    Some(request_body),
    @async-core.Future::ready(Ok(None)),
    None,
  )
  guard request.set_method(@types.Method::Post) is Ok(_) else { panic() }
  guard request.set_path_with_query(Some("/hooks/build")) is Ok(_) else {
    panic()
  }

  let response = match @handler.handle(request) {
    Ok(response) => response
    Err(_) => panic()
  }
  guard response.get_status_code() == 202U else { panic() }

  let (response_body, response_trailers) = response.consume_body(
    @async-core.Future::ready(Ok(())),
  )
  guard response_body.read(1) is None else { panic() }
  response_body.drop()
  match response_trailers.get() {
    Ok(None) => ()
    Ok(Some(fields)) => {
      fields.drop()
      panic()
    }
    Err(_) => panic()
  }

  match request_transmitted.get() {
    Err(@types.ErrorCode::InternalError(Some(message))) => {
      guard message == "unexpected hook payload" else { panic() }
    }
    _ => panic()
  }
}
