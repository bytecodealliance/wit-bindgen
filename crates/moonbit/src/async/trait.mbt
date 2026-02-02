///|
pub(all) struct FutureR[X] {
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
  read : async (Int) -> ArrayView[X]?
  close : async () -> Unit
  take_handle : () -> Int
}

///|
pub fn[X] StreamR::take_handle(self : StreamR[X]) -> Int {
  (self.take_handle)()
}

///|
pub(all) struct StreamW[X] {
  write : async (ArrayView[X]) -> Int
  close : async () -> Unit
}

///|
pub(all) struct OutStreamInner[X] {
  mut stream : StreamW[X]?
  mut coroutine : Coroutine?
}

///|
pub(all) struct OutStream[X] {
  inner : Ref[OutStreamInner[X]]
}

///|
pub fn[X] OutStream::new() -> OutStream[X] {
  { inner: { val: { stream: None, coroutine: None } } }
}

///|
pub async fn[X] OutStream::get_stream(self : OutStream[X]) -> StreamW[X] {
  if self.inner.val.stream is Some(s) {
    return s
  } else {
    guard self.inner.val.coroutine is None
    self.inner.val.coroutine = Some(current_coroutine())
    suspend() catch {
      e => {
        if self.inner.val.stream is Some(s) {
          (s.close)()
        }
        raise e
      }
    }
    self.inner.val.stream.unwrap()
  }
}

///|
pub fn[X] OutStream::put_stream(
  self : OutStream[X],
  stream : StreamW[X],
) -> Unit {
  self.inner.val.stream = Some(stream)
  if self.inner.val.coroutine is Some(coro) {
    coro.wake()
  }
}

///|
pub(all) struct OutFutureInner[X] {
  mut value : X?
  mut coroutine : Coroutine?
}

///|
pub(all) struct OutFuture[X] {
  inner : Ref[OutFutureInner[X]]
}

///|
pub fn[X] OutFuture::new() -> OutFuture[X] {
  { inner: { val: { value: None, coroutine: None } } }
}

///|
pub async fn[X] OutFuture::get(self : OutFuture[X]) -> X {
  if self.inner.val.value is Some(v) {
    return v
  } else {
    guard self.inner.val.coroutine is None
    self.inner.val.coroutine = Some(current_coroutine())
    suspend()
    self.inner.val.value.unwrap()
  }
}

///|
pub fn[X] OutFuture::put(self : OutFuture[X], value : X) -> Unit {
  self.inner.val.value = Some(value)
  if self.inner.val.coroutine is Some(coro) {
    coro.wake()
  }
}
