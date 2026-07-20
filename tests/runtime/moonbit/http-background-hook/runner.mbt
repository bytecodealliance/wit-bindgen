//@ wasmtime-flags = '-Wcomponent-model-async -S p3 -S http'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "import": [{ "path": "example/background-hook/async-core", "alias": "async-core" }, { "path": "example/background-hook/interface/wasi/http/handler", "alias": "handler" }, { "path": "example/background-hook/interface/wasi/http/types", "alias": "types" }] }"""

///|
async fn run_hook(payload : Bytes, expected_error : String?) -> Unit {
  let request_body = @async-core.Stream::produce(async fn(sink) {
    guard sink.write_all_bytes(payload[:]) else {
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
  let expected_response = b"accepted"
  let received = Ref(0)
  for ;; {
    match response_body.read(4) {
      Some(chunk) =>
        for byte in chunk {
          guard received.val < expected_response.length() else { panic() }
          guard byte == expected_response[received.val] else { panic() }
          received.val += 1
        }
      None => break
    }
  }
  guard received.val == expected_response.length() else { panic() }
  response_body.drop()
  match response_trailers.get() {
    Ok(Some(fields)) => fields.drop()
    Ok(None) | Err(_) => panic()
  }

  match expected_error {
    None =>
      match request_transmitted.get() {
        Ok(_) => ()
        Err(_) => panic()
      }
    Some(expected) =>
      match request_transmitted.get() {
        Err(@types.ErrorCode::InternalError(Some(message))) => {
          guard message == expected else { panic() }
        }
        _ => panic()
      }
  }
}

///|
pub async fn run(_background_group : @async-core.TaskGroup[Unit]) -> Unit {
  run_hook(b"build=release", None)
  run_hook(b"build=debug", Some("unexpected hook payload"))
}
