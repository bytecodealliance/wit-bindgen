pub struct Subtask {
  handle : Int
  mut code : Int?
}


pub fn Subtask::from_handle(handle : Int) -> Subtask {
  {handle: handle, code: None}
}

pub impl Waitable for Subtask with update(self, code~ : Int) -> Unit {
  self.code = Some(code)
}

pub impl Waitable for Subtask with handle(self) -> Int {
  self.handle
}

pub impl Waitable for Subtask with cancel(self) -> Unit {
  self.code = Some(subtask_cancel(self.handle))
}

pub impl Waitable for Subtask with drop(self) -> Unit {
  subtask_drop(self.handle)
}

pub impl Waitable for Subtask with is_done(self) -> Bool {
  guard self.code is Some(code) else {
    return false
  }
  match CallbackCode::decode(code) {
    Exit | Cancel(_) => true
    Yield | Wait(_) => false
  }
}
