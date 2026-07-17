//@ [lang]
//@ path = 'gen/interface/my/test/leaf-interface/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "import": [{ "path": "my/test/async-core", "alias": "async-core" }, { "path": "moonbitlang/core/encoding/utf8", "alias": "utf8" }] }"""

///|
priv struct BodyState {
  mut contents : @async-core.Stream[Byte]?
  mut trailers : @async-core.Future[Fields]?
}

///|
priv struct ResponseState {
  mut body : BodyState?
}

///|
let leaf_values : Map[Int, String] = Map([])

///|
let fields_values : Map[Int, String] = Map([])

///|
let body_values : Map[Int, BodyState] = Map([])

///|
let response_values : Map[Int, ResponseState] = Map([])

///|
let next_resource_rep : Ref[Int] = Ref(1)

///|
let leaf_drops : Ref[UInt] = { val: 0 }

///|
let leaf_live : Ref[UInt] = { val: 0 }

///|
let fields_live : Ref[UInt] = { val: 0 }

///|
let body_live : Ref[UInt] = { val: 0 }

///|
let response_live : Ref[UInt] = { val: 0 }

///|
pub fn LeafThing::leaf_thing(value : String) -> LeafThing {
  let rep = next_resource_rep.val
  next_resource_rep.val += 1
  leaf_values[rep] = value
  leaf_live.val += 1
  LeafThing::new(rep)
}

///|
pub fn LeafThing::get(self : LeafThing) -> String {
  leaf_values[self.0]
}

///|
pub fn LeafThing::dtor(self : LeafThing) -> Unit {
  leaf_values.remove(self.0)
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
  let rep = next_resource_rep.val
  next_resource_rep.val += 1
  fields_values[rep] = value
  fields_live.val += 1
  Fields::new(rep)
}

///|
pub fn Fields::get(self : Fields) -> String {
  fields_values[self.0]
}

///|
pub fn Fields::dtor(self : Fields) -> Unit {
  fields_values.remove(self.0)
  fields_live.val -= 1
}

///|
pub fn Body::body(
  contents : @async-core.Stream[Byte],
  trailers : @async-core.Future[Fields]?,
) -> Body {
  let rep = next_resource_rep.val
  next_resource_rep.val += 1
  body_values[rep] = { contents: Some(contents), trailers }
  body_live.val += 1
  Body::new(rep)
}

///|
async fn collect_body_state(state : BodyState) -> (String, String?) {
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
      let value = fields_values[fields.rep()]
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
  collect_body_state(body_values[self.0])
}

///|
pub fn Body::dtor(self : Body) -> Unit {
  match body_values.get(self.0) {
    Some(state) => {
      body_values.remove(self.0)
      guard state.contents is None && state.trailers is None else {
        abort("test body dropped before collect")
      }
      body_live.val -= 1
    }
    None => ()
  }
}

///|
pub fn Response::response(body : Body) -> Response {
  // Move the owned body's representation into the response before releasing
  // its handle. The body remains live until `Response::collect` consumes it.
  let body_rep = body.rep()
  guard body_values.get(body_rep) is Some(body_state) else { panic() }
  body_values.remove(body_rep)
  body.drop()
  let rep = next_resource_rep.val
  next_resource_rep.val += 1
  response_values[rep] = { body: Some(body_state) }
  response_live.val += 1
  Response::new(rep)
}

///|
pub async fn Response::collect(
  self : Response,
  _background_group : @async-core.TaskGroup[Unit],
) -> (String, String?) {
  let state = response_values[self.0]
  guard state.body is Some(body_state) else { panic() }
  state.body = None
  let result = collect_body_state(body_state)
  body_live.val -= 1
  result
}

///|
pub fn Response::dtor(self : Response) -> Unit {
  let state = response_values[self.0]
  guard state.body is None else {
    abort("test response dropped before collect")
  }
  response_values.remove(self.0)
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
