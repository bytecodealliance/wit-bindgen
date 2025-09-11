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
pub fn[T] new_future(
  vtable : FutureVTable[T],
) -> (FutureReader[T], FutureWriter[T]) {
  let handle = (vtable.new)()
  let left_handle = handle.to_int()
  let right_handle = left_handle >> 32
  (
    FutureReader::new(left_handle, vtable),
    FutureWriter::new(right_handle, vtable),
  )
}

///|
pub struct FutureReader[T] {
  handle : Int
  vtable : FutureVTable[T]
  mut code : Int?
}

///|
pub fn[T] FutureReader::new(
  handle : Int,
  vtable : FutureVTable[T],
) -> FutureReader[T] {
  { handle, vtable, code: None }
}

///|
pub impl[T] Waitable for FutureReader[T] with update(self, code~ : Int) -> Unit {
  self.code = Some(code)
}

///|
pub impl[T] Waitable for FutureReader[T] with handle(self) -> Int {
  self.handle
}

///|
pub impl[T] Waitable for FutureReader[T] with cancel(self) -> Unit {
  self.code = Some((self.vtable.cancel_read)(self.handle))
  
}

///|
pub impl[T] Waitable for FutureReader[T] with drop(self) -> Unit {
  (self.vtable.drop_readable)(self.handle)
}

///|
pub impl[T] Waitable for FutureReader[T] with is_done(self) -> Bool {
  match self.code {
    Some(c) =>
      match WaitableStatus::decode(c) {
        Completed(_) | Dropped(_) | Cancelled(_) => true
        Blocking => false
      }
    None => false
  }
}


///|
pub fn[T] FutureReader::malloc(self : FutureReader[T]) -> Int {
  (self.vtable.malloc)()
}

///|
pub fn[T] FutureReader::free(self : FutureReader[T], ptr : Int) -> Unit {
  (self.vtable.free)(ptr)
}

///|
pub fn[T] FutureReader::lift(self : FutureReader[T], ptr : Int) -> T {
  (self.vtable.lift)(ptr)
}

///|
pub fn[T] FutureReader::lower_read(self : FutureReader[T], ptr : Int) -> Int {
  (self.vtable.read)(self.handle, ptr)
}

///|
pub async fn[T] FutureReader::read(self : FutureReader[T]) -> T raise {
  let buf_ptr = self.malloc()
  self.code = Some(self.lower_read(buf_ptr))
  defer self.free(buf_ptr)
  let status = WaitableStatus::decode(self.code.unwrap())
  match status {
    Completed(_) => {
      let value = self.lift(buf_ptr)
      return value
    }

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
  let value = self.lift(buf_ptr)
  return value
}

///|
pub async fn[T] FutureReader::read_then_drop(self : FutureReader[T]) -> T raise {
  defer self.drop()
  let x = self.read() // defer bug
  return x // self.read()
}

///|
pub struct FutureWriter[T] {
  handle : Int
  vtable : FutureVTable[T]
  mut code : Int?
}

///|
pub fn[T] FutureWriter::new(
  handle : Int,
  vtable : FutureVTable[T],
) -> FutureWriter[T] {
  { handle, vtable, code: None }
}

///|
pub impl[T] Waitable for FutureWriter[T] with update(self, code~ : Int) -> Unit {
  self.code = Some(code)
}

///|
pub impl[T] Waitable for FutureWriter[T] with handle(self) -> Int {
  self.handle
}

///|
pub impl[T] Waitable for FutureWriter[T] with cancel(self) -> Unit {
  self.code = Some((self.vtable.cancel_write)(self.handle))
}

///|
pub impl[T] Waitable for FutureWriter[T] with drop(self) -> Unit {
  (self.vtable.drop_writable)(self.handle)
}

///|
pub impl[T] Waitable for FutureWriter[T] with is_done(self) -> Bool {
  match self.code {
    Some(c) =>
      match WaitableStatus::decode(c) {
        Completed(_) | Dropped(_) | Cancelled(_) => true
        Blocking => false
      }
    None => false
  }
}


///|
pub fn[T] FutureWriter::malloc(self : FutureWriter[T]) -> Int {
  (self.vtable.malloc)()
}

///|
pub fn[T] FutureWriter::free(self : FutureWriter[T], ptr : Int) -> Unit {
  (self.vtable.free)(ptr)
}

///|
pub fn[T] FutureWriter::lower(
  self : FutureWriter[T],
  value : T,
  ptr : Int,
) -> Unit {
  (self.vtable.lower)(value, ptr)
}

///|
pub fn[T] FutureWriter::lower_write(self : FutureWriter[T], ptr : Int) -> Int {
  (self.vtable.write)(self.handle, ptr)
}

///|
pub async fn[T] FutureWriter::write(
  self : FutureWriter[T],
  value : T,
) -> Unit raise {
  let buf_ptr = self.malloc()
  self.lower(value, buf_ptr)
  self.code = Some(self.lower_write(buf_ptr))
  defer self.free(buf_ptr)
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
pub async fn[T] FutureWriter::write_then_drop(
  self : FutureWriter[T],
  value : T,
) -> Unit raise {
  defer self.drop()
  self.write(value) 
  return
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

pub fn[T] Stream::new(handle: Int, vtable : StreamVTable[T]) -> Stream[T] {
      { handle, vtable, mode: None }
}