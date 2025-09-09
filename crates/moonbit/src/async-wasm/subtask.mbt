pub struct Subtask {
  handle : Int
  defer_task : Array[() -> Unit raise]
  mut code : Int?
}

pub fn Subtask::add_defer(self : Self, f : () -> Unit raise) -> Unit {
  self.defer_task.push(f)
}

pub fn Subtask::from_handle(handle : Int) -> Subtask {
  {handle: handle, defer_task: [], code: None}
}

pub impl Waitable for Subtask with update(self, code~ : Int) -> Unit {
  self.code = Some(code)
}

pub impl Waitable for Subtask with handle(self) -> Int {
  self.handle
}

pub impl Waitable for Subtask with cancel(self) -> Int {
  subtask_cancel(self.handle)
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

pub impl Waitable for Subtask with defer_task(self) -> Unit raise {
  self.defer_task.each(f => f())
}
