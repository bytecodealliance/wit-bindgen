///|
pub(all) struct FutureR[X] {
  get : async () -> X
  drop : async () -> Unit
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
pub(all) struct StreamR[X] {
  read : async (Int) -> ArrayView[X]?
  close : async () -> Unit
}

///|
pub(all) struct StreamW[X] {
  write : async (ArrayView[X]) -> Int
  close : async () -> Unit
}

///|
pub(all) struct OutStream[X] {
  mut stream : StreamW[X]?
  mut coroutine : Coroutine?
} derive(Default)

///|
pub async fn[X] OutStream::get_stream(self : OutStream[X]) -> StreamW[X] {
  if self.stream is Some(s) {
    return s
  } else {
    guard self.coroutine is None
    self.coroutine = Some(current_coroutine())
    suspend() catch {
      e => {
        if self.stream is Some(s) {
          (s.close)()
        }
        raise e
      }
    }
    self.stream.unwrap()
  }
}

///|
pub fn[X] OutStream::put_stream(
  self : OutStream[X],
  stream : StreamW[X],
) -> Unit {
  self.stream = Some(stream)
  if self.coroutine is Some(coro) {
    coro.wake()
  }
}

///|
pub(all) struct OutFuture[X] {
  mut value : X?
  mut coroutine : Coroutine?
} derive(Default)

///|
pub async fn[X] OutFuture::get(self : OutFuture[X]) -> X {
  if self.value is Some(v) {
    return v
  } else {
    guard self.coroutine is None
    self.coroutine = Some(current_coroutine())
    suspend()
    self.value.unwrap()
  }
}

///|
pub fn[X] OutFuture::put(self : OutFuture[X], value : X) -> Unit {
  self.value = Some(value)
  if self.coroutine is Some(coro) {
    coro.wake()
  }
}
