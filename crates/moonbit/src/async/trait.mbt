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
/// Writes the entire slice to the sink unless the sink closes first.
///
/// Returns `true` if all items were written and `false` if the sink stopped
/// accepting data before completion.
pub async fn[X] Sink::write_all(self : Sink[X], data : ArrayView[X]) -> Bool {
  if data.length() == 0 {
    return true
  }
  let pending : Array[X] = []
  for i = 0; i < data.length(); i = i + 1 {
    pending.push(data[i])
  }
  let mut offset = 0
  for ;; {
    if offset >= pending.length() {
      return true
    }
    let written = self.write(pending[offset:])
    if written <= 0 {
      return false
    }
    offset = offset + written
  }
}

///|
pub async fn[X] Sink::close(self : Sink[X]) -> Unit {
  (self.close)()
}

///|
pub struct Promise[X] {
  write : async (X) -> Unit
  close : async () -> Unit
}

///|
pub async fn[X] Promise::write(self : Promise[X], value : X) -> Unit {
  (self.write)(value)
}

///|
pub async fn[X] Promise::close(self : Promise[X]) -> Unit {
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
///|
priv struct Waiter {
  mut coro : Coroutine?
}

///|
fn wake_one(waiters : @deque.Deque[Waiter]) -> Unit {
  while waiters.pop_front() is Some(waiter) {
    match waiter.coro {
      None => ()
      Some(coro) => {
        coro.wake()
        return
      }
    }
  }
}

///|
fn wake_all(waiters : @deque.Deque[Waiter]) -> Unit {
  while waiters.pop_front() is Some(waiter) {
    match waiter.coro {
      None => ()
      Some(coro) => coro.wake()
    }
  }
}

///|
async fn wait_on(waiters : @deque.Deque[Waiter]) -> Unit {
  let waiter = { coro: Some(current_coroutine()) }
  waiters.push_back(waiter)
  try suspend() catch {
    err => {
      waiter.coro = None
      raise err
    }
  }
}

///|
struct LocalFutureState[X] {
  mut value : X?
  mut closed : Bool
  waiters : @deque.Deque[Waiter]
}

///|
fn[X] local_future_close(state : Ref[LocalFutureState[X]]) -> Unit {
  if state.val.closed {
    return
  }
  state.val.closed = true
  wake_all(state.val.waiters)
}

///|
fn[X] local_future_write(state : Ref[LocalFutureState[X]], value : X) -> Unit {
  if state.val.closed {
    panic()
  }
  if state.val.value is Some(_) {
    panic()
  }
  state.val.value = Some(value)
  wake_all(state.val.waiters)
}

///|
async fn[X] local_future_get(state : Ref[LocalFutureState[X]]) -> X {
  for ;; {
    match state.val.value {
      Some(value) => return value
      None =>
        if state.val.closed {
          raise Cancelled::Cancelled
        } else {
          wait_on(state.val.waiters)
        }
    }
  }
}

///|
struct LocalStreamState[X] {
  capacity : Int
  chunks : @deque.Deque[Array[X]]
  mut buffered : Int
  mut closed : Bool
  mut head : Array[X]?
  mut head_pos : Int
  readers : @deque.Deque[Waiter]
  writers : @deque.Deque[Waiter]
}

///|
fn[X] local_stream_close(state : Ref[LocalStreamState[X]]) -> Unit {
  if state.val.closed {
    return
  }
  state.val.closed = true
  wake_all(state.val.readers)
  wake_all(state.val.writers)
}

///|
async fn[X] local_stream_write(
  state : Ref[LocalStreamState[X]],
  data : ArrayView[X],
) -> Int {
  if data.length() == 0 || state.val.closed {
    return 0
  }
  let total = data.length()
  let mut offset = 0
  for ;; {
    if offset >= total {
      return total
    }
    if state.val.closed {
      return offset
    }
    let capacity = state.val.capacity
    let available = if capacity <= 0 {
      total - offset
    } else {
      capacity - state.val.buffered
    }
    if available <= 0 {
      wait_on(state.val.writers)
      continue
    }
    let take = if available < (total - offset) {
      available
    } else {
      total - offset
    }
    let chunk : Array[X] = []
    for i = 0; i < take; i = i + 1 {
      chunk.push(data[offset + i])
    }
    state.val.chunks.push_back(chunk)
    state.val.buffered = state.val.buffered + take
    offset = offset + take
    wake_one(state.val.readers)
  }
  total
}

///|
async fn[X] local_stream_read(
  state : Ref[LocalStreamState[X]],
  count : Int,
) -> ArrayView[X]? {
  if count <= 0 {
    return Some([])
  }
  let result : Array[X] = []
  for ;; {
    if result.length() >= count {
      return Some(result[:])
    }
    match state.val.head {
      Some(head) => {
        let available = head.length() - state.val.head_pos
        let needed = count - result.length()
        let take = if needed < available { needed } else { available }
        for i = 0; i < take; i = i + 1 {
          result.push(head[state.val.head_pos + i])
        }
        state.val.head_pos = state.val.head_pos + take
        state.val.buffered = state.val.buffered - take
        if state.val.head_pos >= head.length() {
          state.val.head = None
          state.val.head_pos = 0
        }
        wake_one(state.val.writers)
        continue
      }
      None => ()
    }
    if state.val.chunks.pop_front() is Some(chunk) {
      state.val.head = Some(chunk)
      state.val.head_pos = 0
      continue
    }
    if result.length() > 0 {
      return Some(result[:])
    }
    if state.val.closed {
      return None
    }
    wait_on(state.val.readers)
  }
}

///|
pub(all) enum Future[X] {
  Handle(FutureR[X])
  Local(Int, Ref[LocalFutureState[X]])
}

///|
pub impl[X] Eq for Future[X] with equal(self, other) -> Bool {
  match self {
    Handle(f) =>
      match other {
        Handle(g) => f.handle == g.handle
        Local(_) => false
      }
    Local(id, _) =>
      match other {
        Handle(_) => false
        Local(other_id, _) => id == other_id
      }
  }
}

///|
pub impl[X] Show for Future[X] with output(self, logger) {
  match self {
    Handle(f) => {
      logger.write_string("Future::Handle(")
      logger.write_string(f.handle.to_string())
      logger.write_string(")")
    }
    Local(id, _) => {
      logger.write_string("Future::Local(")
      logger.write_string(id.to_string())
      logger.write_string(")")
    }
  }
}

///|
pub fn[X] Future::new() -> (Future[X], Promise[X]) {
  let state : Ref[LocalFutureState[X]] = {
    val: {
      value: None,
      closed: false,
      waiters: @deque.new(),
    },
  }
  let future = Future::Local(fresh_id(), state)
  let promise = Promise::{
    write: (value : X) => { local_future_write(state, value) },
    close: () => { local_future_close(state) },
  }
  (future, promise)
}

///|
pub fn[X] Future::ready(value : X) -> Future[X] {
  let state : Ref[LocalFutureState[X]] = {
    val: {
      value: Some(value),
      closed: false,
      waiters: @deque.new(),
    },
  }
  Future::Local(fresh_id(), state)
}

///|
pub async fn[X] Future::get(self : Future[X]) -> X {
  match self {
    Handle(f) => f.get()
    Local(_, state) => local_future_get(state)
  }
}

///|
pub async fn[X] Future::drop(self : Future[X]) -> Unit {
  match self {
    Handle(f) => f.drop()
    Local(_, state) => local_future_close(state)
  }
}

///|
pub(all) enum Stream[X] {
  Handle(StreamR[X])
  Local(Int, Ref[LocalStreamState[X]])
  Outgoing(Int, Ref[(async (Sink[X]) -> Unit)?])
}

///|
pub impl[X] Eq for Stream[X] with equal(self, other) -> Bool {
  match self {
    Handle(s) =>
      match other {
        Handle(t) => s.handle == t.handle
        Local(_, _) => false
        Outgoing(_, _) => false
      }
    Local(id, _) =>
      match other {
        Handle(_) => false
        Local(other_id, _) => id == other_id
        Outgoing(_, _) => false
      }
    Outgoing(id, _) =>
      match other {
        Handle(_) => false
        Local(_, _) => false
        Outgoing(other_id, _) => id == other_id
      }
  }
}

///|
pub impl[X] Show for Stream[X] with output(self, logger) {
  match self {
    Handle(s) => {
      logger.write_string("Stream::Handle(")
      logger.write_string(s.handle.to_string())
      logger.write_string(")")
    }
    Local(id, _) => {
      logger.write_string("Stream::Local(")
      logger.write_string(id.to_string())
      logger.write_string(")")
    }
    Outgoing(id, _) => {
      logger.write_string("Stream::Outgoing(")
      logger.write_string(id.to_string())
      logger.write_string(")")
    }
  }
}

///|
pub fn[X] Stream::new(capacity? : Int = 0) -> (Stream[X], Sink[X]) {
  let cap = if capacity < 0 { 0 } else { capacity }
  let state : Ref[LocalStreamState[X]] = {
    val: {
      capacity: cap,
      chunks: @deque.new(),
      buffered: 0,
      closed: false,
      head: None,
      head_pos: 0,
      readers: @deque.new(),
      writers: @deque.new(),
    },
  }
  let stream = Stream::Local(fresh_id(), state)
  let sink = Sink::{
    write: (data : ArrayView[X]) => { local_stream_write(state, data) },
    close: () => { local_stream_close(state) },
  }
  (stream, sink)
}

///|
pub fn[X] Stream::from(producer : async (Sink[X]) -> Unit) -> Stream[X] {
  Stream::Outgoing(fresh_id(), { val: Some(producer) })
}

pub async fn[X] Stream::read(self : Stream[X], count : Int) -> ArrayView[X]? {
  match self {
    Handle(s) => s.read(count)
    Local(_, state) => local_stream_read(state, count)
    Outgoing(_, _) => panic()
  }
}

///|
pub async fn[X] Stream::close(self : Stream[X]) -> Unit {
  match self {
    Handle(s) => s.close()
    Local(_, state) => local_stream_close(state)
    Outgoing(_, _) => ()
  }
}
