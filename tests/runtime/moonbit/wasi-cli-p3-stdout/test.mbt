//@ [lang]
//@ path = 'gen/interface/my/test/i/stub.mbt'
//@ pkg_config = """{
//@   "supported-targets": "+wasm",
//@   "import": [
//@     { "path": "my/test/async-core", "alias": "async-core" },
//@     { "path": "my/test/interface/wasi/cli/stdout", "alias": "stdout" }
//@   ]
//@ }"""

///|
pub async fn write_stdout(
  _background_group : @async-core.TaskGroup[Unit],
) -> Result[Unit, Unit] {
  let data : FixedArray[Byte] = [
    b'h', b'e', b'l', b'l', b'o', b' ', b'p', b'3', b'\n',
  ]
  let done = @stdout.write_via_stream(
    @async-core.Stream::produce(async fn(sink) {
      let _ = sink.write_all(data[:])
      sink.close()
    }),
  )
  let result = done.get() catch { _ => return Err(()) }
  match result {
    Ok(_) => Ok(())
    Err(_) => Err(())
  }
}
