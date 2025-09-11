///|
enum TaskStatus {
  Fail(Error)
  Running
  Done
}

///|
pub struct Task {
  id : Int
  tasks : Map[Int, (&Waitable, Coroutine)]
  task_defer : Array[() -> Unit raise]
  mut status : TaskStatus
} // TaskGroup

///|
pub let task_map : Map[Int, Task] = {}

///|
/// this avoids multiple lookups of current waitable task context
let curr_task : Ref[Task?] = Ref::new(None)

///|
pub fn Task::new() -> Task {
  let waitable_set = waitable_set_new()
  activate_waitable_task_context(waitable_set)
  { id: waitable_set, tasks: {}, task_defer: [], status: Running }
}

///|
pub fn Task::from_raw(raw : Int) -> Task {
  guard raw != 0
  activate_waitable_task_context(raw)
  { id: raw, tasks: {}, task_defer: [], status: Running }
}


pub fn Task::is_done(self : Self) -> Bool {
  match self.status {
    Running => false
    Done | Fail(_) => true
  }
}
pub fn Task::is_fail(self: Self) -> Error? {
  match self.status {
    Fail(err) => Some(err)
    _ => None
  }
}

///|
pub fn Task::blocking_wait(self : Self) -> (Int, Int, Int) {
  let result : FixedArray[Int] = FixedArray::make(2, 0)
  let result_ptr = int_array2ptr(result)
  let event0 = waitable_set_wait(self.id, result_ptr)
  (event0, result[0], result[1])
}

///|
pub fn Task::poll(self : Self) -> (Int, Int, Int) {
  let result : FixedArray[Int] = FixedArray::make(2, 0)
  let result_ptr = int_array2ptr(result)
  let event0 = waitable_set_poll(self.id, result_ptr)
  (event0, result[0], result[1])
}

///|
fn[T : Waitable] Task::add_waitable(self : Self, waitable : T) -> Unit {
  waitable_join(waitable.handle(), self.id)
}

///|
/// When a waitable is done will be removed from the waitable set
/// then waitable will be dropped
fn[T : Waitable] Task::remove_waitable(self : Self, waitable : T) -> Unit {
  waitable_join(waitable.handle(), self.id)
  let waitable = self.tasks.get(waitable.handle())
  guard waitable is Some((waitable, _)) else { return }
  waitable.drop()
  self.tasks.remove(waitable.handle())
}

///|
/// Drop the waitable set and all waitables in it
pub fn Task::drop(self : Self) -> Unit {
  activate_waitable_task_context(0) // clear current context
  // remove all waitables and drop them
  for task in self.tasks.values() {
    task.0.drop()
  }
  self.tasks.clear()
  waitable_set_drop(self.id) // runner will drop the waitable set
}

///|
pub fn Task::spawn(
  task : Self,
  f : async () -> Unit raise,
  state : &Waitable,
) -> Unit {
  let coro = spawn(fn() {
    task.add_waitable(state)
    defer task.remove_waitable(state)
    f()
  })
  task.tasks[state.handle()] = (state, coro)
}

///|
/// This function spawns a coroutine to run the async function and waits for its completion
pub async fn Task::wait(
  task : Self,
  f : async () -> Unit raise,
  state : &Waitable,
) -> Unit raise {
  let coro = spawn(fn() {
    task.add_waitable(state)
    defer task.remove_waitable(state)
    f()
  })
  task.tasks[state.handle()] = (state, coro)
  Coroutine::wait(coro)
}

///|
pub fn Task::add_defer(self : Self, f : () -> Unit raise) -> Unit {
  self.task_defer.push(f)
}

pub fn callback(event : Int, waitable_id : Int, code : Int) -> Int {
  let event = Event::decode(event)
  // Handle the event for the current waitable task  
  match event {
    FutureRead | FutureWrite | StreamRead | StreamWrite | Subtask => {
      guard current_waitable_set() is Some(task)
      let (waitable, coro) = task.tasks[waitable_id]
      waitable.update(code~)
      coro.run()
      // schedule next coroutine
      rschedule()
      if waitable.is_done() {
        return CallbackCode::Exit.encode()
      }
      return CallbackCode::Wait(waitable_id).encode()
    }
    TaskCancel => {
      guard current_waitable_set() is Some(task)
      task.tasks.values().each(task => {
        let _ = task.0.cancel()
      })
      defer task.drop()
      rschedule()
      return CallbackCode::Exit.encode()
    }
    None => {
      rschedule()
      return CallbackCode::Exit.encode()
    }
  }
}
///|
pub fn Task::with_waitable_set(
  self : Self,
  f : async (Self) -> Unit raise,
  is_drop? : Bool = false,
) -> Unit noraise {
  let parent = spawn(async fn() -> Unit raise {
    self.status = Running
    defer (if self.tasks.is_empty() {
      self.status = Done
      task_map.remove(self.id)
      curr_task.val = None
      // runner will drop the waitable set
      // export async function needs to keep the waitable set
      if is_drop {
        self.drop()
      }

      // this defer block recycles waitable task resources
      while self.task_defer.pop() is Some(defer_block) {
        defer_block() catch {
          err => if self.status is Done { self.status = Fail(err) }
        }
      }
    })
    f(self)
  })
  parent.run()
  rschedule() // start the parent coroutine 
}

///|
pub fn current_waitable_set() -> Task? {
  guard curr_task.val is Some(ctx) else { task_map.get(context_get()) }
  Some(ctx)
}

///|
pub fn get_or_create_waitable_set() -> Task {
  let ctx = match current_waitable_set() {
    Some(ctx) => ctx
    None => {
      let ctx = Task::new()
      task_map[ctx.id] = ctx
      ctx
    }
  }
  curr_task.val = Some(ctx)
  ctx
}

pub fn create_waitable_set() -> Int {
  let ctx = Task::new()
  task_map[ctx.id] = ctx
  ctx.id
}

///|
fn activate_waitable_task_context(ctx : Int) -> Unit {
  context_set(ctx)
}
