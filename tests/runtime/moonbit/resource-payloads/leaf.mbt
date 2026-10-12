//@ [lang]
//@ path = 'gen/interface/my/test/leaf-interface/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "supported-targets": "+wasm", "import": [{ "path": "my/test/async-core", "alias": "async-core" }, { "path": "moonbitlang/core/encoding/utf8", "alias": "utf8" }] }"""

///|
struct Body {
  mut contents : @async-core.Stream[Byte]?
  mut trailers : @async-core.Future[FieldsHandle]?
}

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
async fn collect_body_state(state : Body) -> (String, String?) {
  guard state.contents is Some(contents) else { panic() }
  state.contents = None

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
  contents.drop()

  let trailers = match state.trailers {
    Some(future) => {
      state.trailers = None
      let fields = future.get()
      let value = fields.get()
      fields.drop()
      future.drop()
      Some(value)
    }
    None => None
  }
  (@utf8.decode_lossy(Bytes::from_array(bytes[:])[:]), trailers)
}

///|
pub async fn Body::collect(
  self : Body,
  _background_group : @async-core.TaskGroup[Unit],
) -> (String, String?) {
  collect_body_state(self)
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
  guard self.body is Some(body) else { panic() }
  self.body = None
  let result = body.collect(background_group)
  body.drop()
  result
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
