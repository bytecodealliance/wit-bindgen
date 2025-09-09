///|
pub struct FutureVTable[T] {
  new : () -> UInt64
  read : (Int, Int) -> Int
  write : (Int, Int) -> Int
  cancel_read : (Int) -> Int
  cancel_write : (Int) -> Int
  drop_readable : (Int) -> Unit
  drop_writable : (Int) -> Unit
  malloc : () -> Int
  free : (Int) -> Unit
  lift : (Int) -> T
  lower : (T, Int) -> Unit
}

///|
pub fn[T] FutureVTable::new(
  new : () -> UInt64,
  read : (Int, Int) -> Int,
  write : (Int, Int) -> Int,
  cancel_read : (Int) -> Int,
  cancel_write : (Int) -> Int,
  drop_readable : (Int) -> Unit,
  drop_writable : (Int) -> Unit,
  malloc : () -> Int,
  free : (Int) -> Unit,
  lift : (Int) -> T,
  lower : (T, Int) -> Unit,
) -> FutureVTable[T] {
  {
    new,
    read,
    write,
    cancel_read,
    cancel_write,
    drop_readable,
    drop_writable,
    malloc,
    free,
    lift,
    lower,
  }
}

///|
// pub struct Future[T](Int,FutureVTable[T])
pub struct Future[T] {
  handle : Int
  vtable : FutureVTable[T]
  mut code : Int?
}

///|
pub impl[T] Waitable for FutureReader[T] with update(self, code~ : Int) -> Unit {
  self.0.code = Some(code)
}

///|
pub impl[T] Waitable for FutureReader[T] with handle(self) -> Int {
  self.0.handle
}

///|
pub impl[T] Waitable for FutureReader[T] with cancel(self) -> Int {
  self.0.cancel_read()
}

///|
pub impl[T] Waitable for FutureReader[T] with drop(self) -> Unit {
  self.0.drop_readable()
}

///|
pub impl[T] Waitable for FutureReader[T] with is_done(self) -> Bool {
  match self.0.code {
    Some(c) =>
      match WaitableStatus::decode(c) {
        Completed(_) | Dropped(_) | Cancelled(_) => true
        Blocking => false
      }
    None => false
  }
}

///|
pub impl[T] Waitable for FutureReader[T] with defer_task(self) -> Unit raise {
  self.0.drop_readable()
}

///|
pub impl[T] Waitable for FutureWriter[T] with update(self, code~ : Int) -> Unit {
  self.0.code = Some(code)
}

///|
pub impl[T] Waitable for FutureWriter[T] with handle(self) -> Int {
  self.0.handle
}

///|
pub impl[T] Waitable for FutureWriter[T] with cancel(self) -> Int {
  self.0.cancel_write()
}

///|
pub impl[T] Waitable for FutureWriter[T] with drop(self) -> Unit {
  self.0.drop_writable()
}

///|
pub impl[T] Waitable for FutureWriter[T] with is_done(self) -> Bool {
  match self.0.code {
    Some(c) =>
      match WaitableStatus::decode(c) {
        Completed(_) | Dropped(_) | Cancelled(_) => true
        Blocking => false
      }
    None => false
  }
}

///|
pub impl[T] Waitable for FutureWriter[T] with defer_task(self) -> Unit raise {
  self.0.drop_writable()
}

///|
pub fn[T] Future::new_future(
  vtable : FutureVTable[T],
) -> (Future[T], Future[T]) {
  let handle = (vtable.new)()
  let left_handle = handle.to_int()
  let right_handle = left_handle >> 32
  (Future::new(left_handle, vtable), Future::new(right_handle, vtable))
}

///|
pub fn[T] Future::new(handle : Int, vtable : FutureVTable[T]) -> Future[T] {
  { handle, vtable, code: None }
}

///|
pub fn[T] Future::lower_read(self : Future[T], ptr : Int) -> Int {
  (self.vtable.read)(self.handle, ptr)
}

///|
pub fn[T] Future::lower_write(self : Future[T], ptr : Int) -> Int {
  (self.vtable.write)(self.handle, ptr)
}

///|
pub fn[T] Future::cancel_read(self : Future[T]) -> Int {
  (self.vtable.cancel_read)(self.handle)
}

///|
pub fn[T] Future::cancel_write(self : Future[T]) -> Int {
  (self.vtable.cancel_write)(self.handle)
}

///|
pub fn[T] Future::drop_readable(self : Future[T]) -> Unit {
  (self.vtable.drop_readable)(self.handle)
}

///|
pub fn[T] Future::drop_writable(self : Future[T]) -> Unit {
  (self.vtable.drop_writable)(self.handle)
}

///|
pub fn[T] Future::handle(self : Future[T]) -> Int {
  self.handle
}

///|
pub fn[T] Future::vtable(self : Future[T]) -> FutureVTable[T] {
  self.vtable
}

///|
pub fn[T] Future::to_reader(self : Future[T]) -> FutureReader[T] {
  FutureReader::new(self)
}

///|
pub fn[T] Future::to_writer(self : Future[T]) -> FutureWriter[T] {
  FutureWriter::new(self)
}

///|
pub async fn[T] Future::read(self : Future[T]) -> T raise {
  let buf_ptr = (self.vtable.malloc)()
  self.code = Some(self.lower_read(buf_ptr))
  defer (self.vtable.free)(buf_ptr)
  let status = WaitableStatus::decode(self.code.unwrap())
  match status {
    Completed(_) => return (self.vtable.lift)(buf_ptr)
    Dropped(_) | Cancelled(_) => raise Cancelled::Cancelled
    Blocking => suspend()
  }
  // wait until ready
  for {
    let status = WaitableStatus::decode(self.code.unwrap())
    match status {
      Cancelled(_) | Dropped(_) => raise Cancelled::Cancelled
      Completed(_) => break
      Blocking => suspend()
    }
  }
  // when receive event, continue this coroutine
  return (self.vtable.lift)(buf_ptr)
}

///|
pub async fn[T] Future::write(self : Future[T], value : T) -> Unit raise {
  let buf_ptr = (self.vtable.malloc)()
  (self.vtable.lower)(value, buf_ptr)
  self.code = Some(self.lower_write(buf_ptr))
  defer (self.vtable.free)(buf_ptr)
  let status = WaitableStatus::decode(self.code.unwrap())
  match status {
    Completed(_) => return
    Dropped(_) | Cancelled(_) => raise Cancelled::Cancelled
    Blocking => suspend()
  }
  // wait until ready
  for {
    let status = WaitableStatus::decode(self.code.unwrap())
    match status {
      Cancelled(_) | Dropped(_) => raise Cancelled::Cancelled
      Completed(_) => break
      Blocking => suspend()
    }
  }
  // when receive event, continue this coroutine
  return
}

///|
pub async fn[T] Future::read_then_drop(self : Future[T]) -> T raise {
  defer self.drop_readable()
  let x = self.read() // defer bug
  return x // self.read()
}

///|
pub async fn[T] Future::write_then_drop(
  self : Future[T],
  value : T,
) -> Unit raise {
  defer self.drop_writable()
  return self.write(value)
}

///|
pub struct FutureReader[T](Future[T])

///|
pub struct FutureWriter[T](Future[T])

///|
pub fn[T] FutureReader::new(future : Future[T]) -> FutureReader[T] {
  FutureReader(future)
}

///|
pub fn[T] FutureWriter::new(future : Future[T]) -> FutureWriter[T] {
  FutureWriter(future)
}

///|
pub async fn[T] FutureReader::read(self : FutureReader[T]) -> T raise {
  self.0.read()
}

///|
pub async fn[T] FutureWriter::write(
  self : FutureWriter[T],
  value : T,
) -> Unit raise {
  self.0.write(value)
}



pub struct StreamVTable[T] {
  new : () -> UInt64
  read : (Int, Int, Int) -> Int
  write : (Int, Int, Int) -> Int
  cancel_read : (Int) -> Int
  cancel_write : (Int) -> Int
  drop_readable : (Int) -> Unit
  drop_writable : (Int) -> Unit
  malloc : () -> Int
  free : (Int) -> Unit
  lift : (Int) -> T
  lower : (T, Int) -> Unit
}

///|
pub fn[T] StreamVTable::new(
  new : () -> UInt64,
  read : (Int, Int, Int) -> Int,
  write : (Int, Int, Int) -> Int,
  cancel_read : (Int) -> Int,
  cancel_write : (Int) -> Int,
  drop_readable : (Int) -> Unit,
  drop_writable : (Int) -> Unit,
  malloc : () -> Int,
  free : (Int) -> Unit,
  lift : (Int) -> T,
  lower : (T, Int) -> Unit,
) -> StreamVTable[T] {
  {
    new,
    read,
    write,
    cancel_read,
    cancel_write,
    drop_readable,
    drop_writable,
    malloc,
    free,
    lift,
    lower,
  }
}

///|
// pub struct Future[T](Int,FutureVTable[T])
pub struct Stream[T] {
  handle : Int
  vtable : StreamVTable[T]
  mode : Int?
}

pub fn[T] Stream::new(handle : Int, vtable : StreamVTable[T]) -> Stream[T] {
  { handle, vtable, mode: None }
}