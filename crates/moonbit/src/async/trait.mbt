///|
pub(all) struct FutureR[X] {
  handle : Int
  get : async () -> X
  drop : async () -> Unit
  take_handle : () -> Int
}

///|
pub async fn[X] FutureR::get(self : FutureR[X]) -> X {
  (self.get)()
}

///|
pub async fn[X] FutureR::drop(self : FutureR[X]) -> Unit {
  (self.drop)()
}

///|
pub fn[X] FutureR::take_handle(self : FutureR[X]) -> Int {
  (self.take_handle)()
}

///|
pub(all) struct StreamR[X] {
  handle : Int
  read : async (Int) -> ArrayView[X]?
  close : async () -> Unit
  take_handle : () -> Int
}

///|
pub async fn[X] StreamR::read(self : StreamR[X], count : Int) -> ArrayView[X]? {
  (self.read)(count)
}

///|
pub async fn[X] StreamR::close(self : StreamR[X]) -> Unit {
  (self.close)()
}

///|
pub fn[X] StreamR::take_handle(self : StreamR[X]) -> Int {
  (self.take_handle)()
}

///|
pub(all) struct Sink[X] {
  write : async (ArrayView[X]) -> Int
  close : async () -> Unit
}

///|
pub async fn[X] Sink::write(self : Sink[X], data : ArrayView[X]) -> Int {
  (self.write)(data)
}

///|
pub async fn[X] Sink::close(self : Sink[X]) -> Unit {
  (self.close)()
}

///|
let next_id : Ref[Int] = { val: 0 }

///|
fn fresh_id() -> Int {
  let id = next_id.val
  next_id.val = id + 1
  id
}

///|
pub(all) struct FutureOutInner[X] {
  mut producer : (async () -> X)?
}

///|
pub(all) struct FutureOut[X] {
  id : Int
  inner : Ref[FutureOutInner[X]]
}

///|
pub fn[X] FutureOut::new(producer : async () -> X) -> FutureOut[X] {
  { id: fresh_id(), inner: { val: { producer: Some(producer) } } }
}

///|
pub fn[X] FutureOut::take_producer(self : FutureOut[X]) -> (async () -> X) {
  guard self.inner.val.producer is Some(p)
  self.inner.val.producer = None
  p
}

///|
pub(all) enum Future[X] {
  Incoming(FutureR[X])
  Outgoing(FutureOut[X])
}

///|
pub impl[X] Eq for Future[X] with equal(self, other) -> Bool {
  match self {
    Incoming(f) =>
      match other {
        Incoming(g) => f.handle == g.handle
        Outgoing(_) => false
      }
    Outgoing(f) =>
      match other {
        Incoming(_) => false
        Outgoing(g) => f.id == g.id
      }
  }
}

///|
pub impl[X] Show for Future[X] with output(self, logger) {
  match self {
    Incoming(f) => {
      logger.write_string("Future::Incoming(")
      logger.write_string(f.handle.to_string())
      logger.write_string(")")
    }
    Outgoing(f) => {
      logger.write_string("Future::Outgoing(")
      logger.write_string(f.id.to_string())
      logger.write_string(")")
    }
  }
}

///|
pub fn[X] Future::ready(value : X) -> Future[X] {
  Future::Outgoing(FutureOut::new(async fn() { value }))
}

///|
pub fn[X] Future::from(producer : async () -> X) -> Future[X] {
  Future::Outgoing(FutureOut::new(producer))
}

///|
pub async fn[X] Future::get(self : Future[X]) -> X {
  match self {
    Incoming(f) => f.get()
    Outgoing(f) => (f.take_producer())()
  }
}

///|
pub async fn[X] Future::drop(self : Future[X]) -> Unit {
  match self {
    Incoming(f) => f.drop()
    Outgoing(_) => ()
  }
}

///|
pub(all) struct StreamOutInner[X] {
  mut producer : (async (Sink[X]) -> Unit)?
}

///|
pub(all) struct StreamOut[X] {
  id : Int
  inner : Ref[StreamOutInner[X]]
}

///|
pub fn[X] StreamOut::new(producer : async (Sink[X]) -> Unit) -> StreamOut[X] {
  { id: fresh_id(), inner: { val: { producer: Some(producer) } } }
}

///|
pub fn[X] StreamOut::take_producer(self : StreamOut[X]) -> (async (Sink[X]) -> Unit) {
  guard self.inner.val.producer is Some(p)
  self.inner.val.producer = None
  p
}

///|
pub(all) enum Stream[X] {
  Incoming(StreamR[X])
  Outgoing(StreamOut[X])
}

///|
pub impl[X] Eq for Stream[X] with equal(self, other) -> Bool {
  match self {
    Incoming(s) =>
      match other {
        Incoming(t) => s.handle == t.handle
        Outgoing(_) => false
      }
    Outgoing(s) =>
      match other {
        Incoming(_) => false
        Outgoing(t) => s.id == t.id
      }
  }
}

///|
pub impl[X] Show for Stream[X] with output(self, logger) {
  match self {
    Incoming(s) => {
      logger.write_string("Stream::Incoming(")
      logger.write_string(s.handle.to_string())
      logger.write_string(")")
    }
    Outgoing(s) => {
      logger.write_string("Stream::Outgoing(")
      logger.write_string(s.id.to_string())
      logger.write_string(")")
    }
  }
}

///|
pub fn[X] Stream::from(producer : async (Sink[X]) -> Unit) -> Stream[X] {
  Stream::Outgoing(StreamOut::new(producer))
}

///|
pub async fn[X] Stream::read(self : Stream[X], count : Int) -> ArrayView[X]? {
  match self {
    Incoming(s) => s.read(count)
    Outgoing(_) => panic()
  }
}

///|
pub async fn[X] Stream::close(self : Stream[X]) -> Unit {
  match self {
    Incoming(s) => s.close()
    Outgoing(_) => ()
  }
}
