///|
pub enum TaskStatus {
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

///|
pub fn Task::add_waitable(self : Self, waitable : Int) -> Unit {
  waitable_join(waitable, self.id)
}

///|
pub fn Task::set_wait(self : Self) -> (Int, Int, Int) {
  let result : FixedArray[Int] = FixedArray::make(2, 0)
  let result_ptr = int_array2ptr(result)
  let event0 = waitable_set_wait(self.id, result_ptr)
  (event0, result[0], result[1])
}

///|
pub fn Task::set_poll(self : Self) -> (Int, Int, Int) {
  let result : FixedArray[Int] = FixedArray::make(2, 0)
  let result_ptr = int_array2ptr(result)
  let event0 = waitable_set_poll(self.id, result_ptr)
  (event0, result[0], result[1])
}

///|
pub fn Task::remove_waitable(self : Self, waitable : Int) -> Unit {
  waitable_join(waitable, self.id)
  self.tasks.remove(waitable)
}

///|
pub fn Task::drop(self : Self) -> Unit {
  activate_waitable_task_context(0) // clear current context
  waitable_set_drop(self.id) // runner will drop the waitable set
  self.tasks.clear()
}

pub fn Task::spawn(
  task : Self,
  f : async () -> Unit raise,
  tracing : &Waitable,
) -> Unit {
  let coro = spawn(fn() {
    task.add_waitable(tracing.handle())
    defer task.remove_waitable(tracing.handle())
    f()
  })
  task.tasks[tracing.handle()] = (tracing, coro)
}

///|
/// This function spawns a coroutine to run the async function and waits for its completion
pub async fn Task::wait(
  task : Self,
  f : async () -> Unit raise,
  tracing : &Waitable,
) -> Unit raise {
  let coro = spawn(fn() {
    task.add_waitable(tracing.handle())
    defer task.remove_waitable(tracing.handle())
    f()
  })
  task.tasks[tracing.handle()] = (tracing, coro)
  Coroutine::wait(coro)
}

///|
pub fn Task::add_defer(self : Self, f : () -> Unit raise) -> Unit {
  self.task_defer.push(f)
}

///|
/// This function is core of the wasm async runtime
/// It is called by the host when an event happens
pub fn callback(event : Int, waitable_id : Int, code : Int) -> Int {
  // Handle the event for the current waitable task
  guard current_waitable_set() is Some(task)
  let (waitable, coro) = task.tasks[waitable_id]
  let event = Event::decode(event)
  match event {
    FutureRead | FutureWrite | StreamRead | StreamWrite | Subtask => {
      waitable.update(code~)
      coro.run()
    }
    TaskCancel => {
      let _ = waitable.cancel()
      task_cancel() // TODO: need validate this method
    }
    None => () 
  }
  rschedule()
  if waitable.is_done() {
    return CallbackCode::Exit.encode()
  }
  return CallbackCode::Wait(waitable_id).encode()
}

///|
pub fn Task::with_waitable_set(
  self : Self,
  f : async (Self) -> Unit raise,
  is_drop? : Bool = false,
) -> Unit noraise {
  let parent = spawn(async fn() -> Unit raise {
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

///|
fn activate_waitable_task_context(ctx : Int) -> Unit {
  context_set(ctx)
}
