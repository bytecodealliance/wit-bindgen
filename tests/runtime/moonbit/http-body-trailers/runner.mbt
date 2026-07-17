//@ wasmtime-flags = '-Wcomponent-model-async -S p3 -S http'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "import": [{ "path": "my/test/async-core", "alias": "async-core" }, { "path": "my/test/interface/wasi/http/handler", "alias": "handler" }, { "path": "my/test/interface/wasi/http/types", "alias": "types" }] }"""

///|
pub async fn run(_background_group : @async-core.TaskGroup[Unit]) -> Unit {
  let (request, request_transmitted) = @types.Request::new(
    @types.Fields::fields(),
    None,
    @async-core.Future::ready(Ok(None)),
    None,
  )
  let response = match @handler.handle(request) {
    Ok(response) => response
    Err(_) => panic()
  }

  // Completing this future tells the handler that the response was transmitted.
  let (body, trailers) = @types.Response::consume_body(
    response,
    @async-core.Future::ready(Ok(())),
  )
  let bytes : Array[Byte] = []
  for ;; {
    match body.read(4) {
      Some(chunk) =>
        for i = 0; i < chunk.length(); i = i + 1 {
          bytes.push(chunk[i])
        }
      None => break
    }
  }

  let expected = b"hello http p3"
  guard bytes.length() == expected.length() else { panic() }
  for i = 0; i < expected.length(); i = i + 1 {
    guard bytes[i] == expected[i] else { panic() }
  }

  match trailers.get() {
    Ok(Some(fields)) => fields.drop()
    Ok(None) | Err(_) => panic()
  }
  body.drop()
  request_transmitted.drop()
}
