// Copyright 2025 International Digital Economy Academy
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

///|
/// State owned by one P3 component task. `task.state` is the sole completion
/// source; `owned_coroutines` contains only runtime bridge coroutines.
priv struct ComponentTaskState {
  lane : TaskLane
  subscribers : Map[Int, Subscriber]
  task : Coroutine
  owned_coroutines : @set.Set[Coroutine]
  mut resolution : TaskResolution
  mut cancellation_pending : Bool
}

///|
priv struct TaskWakeup {
  reader : Int
  writer : Int
  mut reading : Bool
  mut signaled : Bool
}

///|
priv struct Subscriber {
  mut event : Events?
  coro : @set.Set[Coroutine]
}

///|
priv enum TaskResolution {
  Pending
  FlushPending
  Resolved
}

///|
fn current_waitableset() -> WaitableSet {
  WaitableSet(tls_get())
}

///|
fn component_task_state(waitable_set : WaitableSet) -> ComponentTaskState? {
  component_tasks.get(waitable_set)
}

///|
fn finish_waitableset(
  waitable_set : WaitableSet,
  task_state : ComponentTaskState,
) -> Unit {
  guard task_state.owned_coroutines.is_empty() else { panic() }
  guard task_state.lane.blocking == 0 && task_state.lane.run_later.is_empty() else {
    panic()
  }
  drop_task_wakeup(task_state)
  guard task_state.subscribers.is_empty() else { panic() }
  component_tasks.remove(waitable_set)
  waitable_set.drop()
  tls_set(0)
}

///|
fn acknowledge_cancellation(task_state : ComponentTaskState) -> Unit {
  guard task_state.cancellation_pending else { return }
  match task_state.task.state {
    Done => () // The generated wrapper already called task.return.
    Fail(_) =>
      if task_state.resolution is Pending {
        task_cancel()
        task_state.resolution = Resolved
      }
    Running | Suspend(_) => return
  }
  task_state.cancellation_pending = false
}

///|
fn notify_subscriber(
  task_state : ComponentTaskState,
  waitable_id : Int,
  event : Events,
) -> Unit {
  if task_state.lane.wakeup is Some(wakeup) && wakeup.reader == waitable_id {
    guard event is StreamRead(i, { progress: 1, copy_result: Completed }) &&
      i == waitable_id
    wakeup.reading = false
    wakeup.signaled = false
    task_state.subscribers.remove(waitable_id)
    waitable_join(waitable_id, 0)
    return
  }
  guard task_state.subscribers.get(waitable_id) is Some(subscriber)
  subscriber.event = Some(event)
  subscriber.coro.each(Coroutine::wake)
  // Keep the delivered event discoverable until its waiter consumes it. A
  // local cancellation can race with the wakeup, and cancelling an already
  // terminal canonical operation would trap.
  waitable_join(waitable_id, 0)
}

///|
fn next_callback(
  waitable_set : WaitableSet,
  task_state : ComponentTaskState,
) -> Int {
  if task_state.resolution is FlushPending {
    task_state.resolution = Resolved
    reschedule(task_state.lane)
  }
  acknowledge_cancellation(task_state)
  if task_state.task.state is (Done | Fail(_)) && no_more_work(task_state.lane) {
    let resolution = task_state.resolution
    finish_waitableset(waitable_set, task_state)
    guard resolution is Resolved
    return CallbackCode::Completed.encode()
  }
  if has_immediately_ready_task(task_state.lane) {
    if !task_state.subscribers.is_empty() {
      let (event, waitable_id) = waitable_set.poll()
      match event {
        None => ()
        TaskCancelled => panic()
        _ => notify_subscriber(task_state, waitable_id, event)
      }
    }
    tls_set(waitable_set.0)
    return CallbackCode::Yield.encode()
  }
  tls_set(waitable_set.0)
  arm_task_wakeup(waitable_set, task_state)
  CallbackCode::Wait(waitable_set.0).encode()
}

///|
#internal(wit_bindgen, "generated binding code only")
#doc(hidden)
pub fn with_waitableset(f : async () -> Unit) -> Int {
  let waitable_set = WaitableSet::new()
  let lane = { blocking: 0, run_later: Deque([]), wakeup: None }
  tls_set(waitable_set.0)
  let task = spawn_owned(f, lane, None)
  let task_state = {
    lane,
    subscribers: Map([]),
    task,
    owned_coroutines: Set([]),
    resolution: Pending,
    cancellation_pending: false,
  }
  component_tasks.set(waitable_set, task_state)
  reschedule(lane)
  next_callback(waitable_set, task_state)
}

///|
/// Record that the generated export wrapper has resolved the component-model
/// task. One additional fair scheduling round lets the enclosing MoonBit task
/// group observe its body completing without draining runnable background work.
#internal(wit_bindgen, "generated binding code only")
#doc(hidden)
pub fn task_returned() -> Unit {
  let waitable_set = current_waitableset()
  guard component_task_state(waitable_set) is Some(task_state)
  guard task_state.resolution is Pending
  task_state.resolution = FlushPending
}

///|
#internal(wit_bindgen, "generated binding code only")
#doc(hidden)
pub fn cb(event : Int, waitable_id : Int, code : Int) -> Int {
  let waitable_set = current_waitableset()
  guard component_task_state(waitable_set) is Some(task_state) else { panic() }
  let events = Events::new(EventCode::from(event), waitable_id, code)
  let preserve_task_wakeup = match events {
    StreamRead(i, _) =>
      match task_state.lane.wakeup {
        Some(wakeup) => wakeup.reader == i
        None => false
      }
    _ => false
  }
  cancel_task_wakeup_read(task_state, preserve=preserve_task_wakeup)
  match events {
    None => {
      reschedule(task_state.lane)
      next_callback(waitable_set, task_state)
    }
    TaskCancelled => {
      task_state.cancellation_pending = true
      task_state.task.cancel()
      task_state.owned_coroutines.each(Coroutine::cancel)
      reschedule(task_state.lane)
      next_callback(waitable_set, task_state)
    }
    _ => {
      notify_subscriber(task_state, waitable_id, events)
      reschedule(task_state.lane)
      next_callback(waitable_set, task_state)
    }
  }
}

///|
let component_tasks : Map[WaitableSet, ComponentTaskState] = Map([])

///|
/// Spawn a coroutine owned by the current component-model async task.
///
/// This is intended for runtime bridge work such as lowering local futures and
/// streams to component-model handles. The coroutine does not participate in
/// `TaskGroup` structured concurrency, but it is cancelled when the current
/// component-model task is cancelled and keeps the waitable-set alive until it
/// terminates.
#internal(wit_bindgen, "generated binding code only")
#doc(hidden)
pub fn spawn_component_task_current(f : async () -> Unit) -> Unit {
  let waitable_set = current_waitableset()
  guard component_task_state(waitable_set) is Some(task_state)
  let coro = spawn_owned(
    async fn() -> Unit {
      let coro = current_coroutine()
      defer task_state.owned_coroutines.remove(coro)
      f()
    },
    task_state.lane,
    Some(spawn_component_task_current),
  )
  task_state.owned_coroutines.add(coro)
}

///|
#internal(wit_bindgen, "generated binding code only")
#doc(hidden)
pub fn has_component_task_scope() -> Bool {
  let waitable_set = current_waitableset()
  component_task_state(waitable_set) is Some(_)
}

///|
#internal(wit_bindgen, "generated binding code only")
#doc(hidden)
pub fn current_component_task_token() -> Int {
  current_waitableset().0
}

///|
fn detach_waitable(task_state : ComponentTaskState, waitable_id : Int) -> Unit {
  if task_state.subscribers.get(waitable_id) is Some(subscriber) {
    subscriber.coro.clear()
    task_state.subscribers.remove(waitable_id)
  }
  waitable_join(waitable_id, 0)
}

///|
fn task_wakeup(lane : TaskLane) -> TaskWakeup {
  match lane.wakeup {
    Some(wakeup) => wakeup
    None => {
      let pair = task_wakeup_stream_new()
      let wakeup = {
        reader: pair.to_int(),
        writer: (pair >> 32).to_int(),
        reading: false,
        signaled: false,
      }
      lane.wakeup = Some(wakeup)
      wakeup
    }
  }
}

///|
fn arm_task_wakeup(
  waitable_set : WaitableSet,
  task_state : ComponentTaskState,
) -> Unit {
  let wakeup = task_wakeup(task_state.lane)
  if wakeup.reading {
    return
  }
  let result = task_wakeup_stream_read(wakeup.reader, 0, 1)
  guard result == -1
  let subscribers = task_state.subscribers
  guard subscribers.get(wakeup.reader) is None
  subscribers.set(wakeup.reader, { event: None, coro: Set([]) })
  waitable_join(wakeup.reader, waitable_set.0)
  wakeup.reading = true
  wakeup.signaled = false
}

///|
fn signal_component_task(lane : TaskLane) -> Unit {
  guard lane.wakeup is Some(wakeup) else { return }
  if !wakeup.reading || wakeup.signaled {
    return
  }
  let result = StreamResult::from(task_wakeup_stream_write(wakeup.writer, 0, 1))
  guard result.progress == 1 && result.copy_result is Completed
  wakeup.signaled = true
}

///|
fn cancel_task_wakeup_read(
  task_state : ComponentTaskState,
  preserve~ : Bool,
) -> Unit {
  guard task_state.lane.wakeup is Some(wakeup) else { return }
  if !wakeup.reading || preserve {
    return
  }
  waitable_join(wakeup.reader, 0)
  task_state.subscribers.remove(wakeup.reader)
  ignore(task_wakeup_stream_cancel_read(wakeup.reader))
  wakeup.reading = false
  wakeup.signaled = false
}

///|
fn drop_task_wakeup(task_state : ComponentTaskState) -> Unit {
  guard task_state.lane.wakeup is Some(wakeup) else { return }
  if wakeup.reading {
    waitable_join(wakeup.reader, 0)
    task_state.subscribers.remove(wakeup.reader)
    ignore(task_wakeup_stream_cancel_read(wakeup.reader))
  }
  task_wakeup_stream_drop_readable(wakeup.reader)
  task_wakeup_stream_drop_writable(wakeup.writer)
  task_state.lane.wakeup = None
}

///|
fn cancel_waitable_event(
  owner_task : Int,
  waitable_id : Int,
  cancel : () -> Events,
) -> Events {
  let waitable_set = WaitableSet(owner_task)
  guard component_task_state(waitable_set) is Some(task_state)
  guard task_state.subscribers.get(waitable_id) is Some(subscriber)

  // Baseline cancel-read is synchronous and traps while its endpoint belongs
  // to a waitable set. Detach before invoking the generated intrinsic.
  waitable_join(waitable_id, 0)
  let event = match subscriber.event {
    Some(event) => event
    None => cancel()
  }
  subscriber.event = Some(event)
  subscriber.coro.each(Coroutine::wake)
  task_state.subscribers.remove(waitable_id)
  event
}

///|
/// Complete an operation immediately or suspend until its terminal waitable
/// event arrives. This is the single owner of subscription registration and
/// detachment for future and stream copy operations.
async fn waitable_event(waitable_id : Int, immediate : Events?) -> Events {
  let waitable_set = current_waitableset()
  let task_state = component_task_state(waitable_set).unwrap()
  let subscribers = task_state.subscribers
  match immediate {
    Some(event) => {
      if subscribers.get(waitable_id) is Some(subscriber) {
        subscriber.event = Some(event)
        subscriber.coro.each(Coroutine::wake)
        subscriber.coro.clear()
        waitable_join(waitable_id, 0)
      }
      subscribers.remove(waitable_id)
      event
    }
    None => {
      let subscriber = if subscribers.get(waitable_id) is Some(subscriber) {
        subscriber
      } else {
        let subscriber = { event: None, coro: Set([]) }
        subscribers.set(waitable_id, subscriber)
        subscriber
      }
      guard subscriber.event is None
      waitable_join(waitable_id, waitable_set.0)
      let coro = current_coroutine()
      subscriber.coro.add(coro)
      defer subscriber.coro.remove(coro)
      let event = try suspend() catch {
        err =>
          match subscriber.event {
            // Completion owns the canonical buffer once its event is
            // delivered, even if local cancellation wakes the same coroutine.
            Some(event) => event
            None => raise err
          }
      } noraise {
        _ => subscriber.event.unwrap()
      }
      subscribers.remove(waitable_id)
      event
    }
  }
}

///|
#internal(wit_bindgen, "generated binding code only")
#doc(hidden)
pub async fn suspend_for_subtask(
  val : Int,
  settle_lowered_arguments : (Bool) -> Unit,
  drop_returned_result : () -> Unit,
) -> Unit {
  let task = SubTask::from(val)
  let mut cleaned = false

  // Once the subtask leaves Starting, canonical arguments are either
  // transferred or rejected. Generated code owns the corresponding action.
  fn settle_arguments(state : SubTaskState) -> Unit {
    if !cleaned && !(state is Starting) {
      cleaned = true
      settle_lowered_arguments(state is Cancelled_before_started)
    }
  }

  // Immediate completion without a handle.
  if task.handle == 0 {
    settle_arguments(task.state)
    match task.state {
      Returned => return
      Cancelled_before_started => raise SubTaskCancelled(before_started=true)
      Cancelled_before_returned => raise SubTaskCancelled(before_started=false)
      _ => panic()
    }
  }

  defer subtask_drop(task.handle)

  // Initial state, return if finished
  settle_arguments(task.state)
  match task.state {
    Returned => return
    Cancelled_before_started => raise SubTaskCancelled(before_started=true)
    Cancelled_before_returned => raise SubTaskCancelled(before_started=false)
    _ => ()
  }

  // A subtask can report intermediate states, so register the same subscriber
  // again after each callback until a terminal state arrives.
  let waitable_set = current_waitableset()
  let task_state = component_task_state(waitable_set).unwrap()
  let set = @set.Set([])
  let subscriber = { event: None, coro: set }
  let coro = current_coroutine()
  set.add(coro)
  defer {
    set.remove(coro)
    detach_waitable(task_state, task.handle)
  }
  for ;; {
    let subscribers = task_state.subscribers
    guard subscribers.get(task.handle) is None
    subscribers.set(task.handle, subscriber)
    waitable_join(task.handle, waitable_set.0)
    suspend() catch {
      Cancelled::Cancelled =>
        // Cancel the subtask
        return protect_from_cancel(() => {
          detach_waitable(task_state, task.handle)
          // A terminal event can race with local cancellation after waking
          // this coroutine. Once delivered, cancelling the subtask again
          // traps, so consume that event before requesting cancellation.
          if subscriber.event is Some(Subtask(i, state)) {
            guard i == task.handle
            settle_arguments(state)
            match state {
              Returned => {
                drop_returned_result()
                return
              }
              Cancelled_before_started | Cancelled_before_returned =>
                raise Cancelled::Cancelled
              Starting | Started => subscriber.event = None
            }
          }
          let state = task.cancel()
          settle_arguments(state)
          match state {
            Returned => {
              drop_returned_result()
              return
            }
            Cancelled_before_started | Cancelled_before_returned =>
              raise Cancelled::Cancelled
            Starting | Started => panic()
          }
        })
      err => raise err
    }

    task_state.subscribers.remove(task.handle)

    // Subsequent state, return if finished
    if subscriber.event is Some(Subtask(i, state)) {
      guard i == task.handle
      settle_arguments(state)
      match state {
        Returned => return
        Cancelled_before_started => raise SubTaskCancelled(before_started=true)
        Cancelled_before_returned =>
          raise SubTaskCancelled(before_started=false)
        _ => subscriber.event = None
      }
    }
  }
}

///|
#internal(wit_bindgen, "generated binding code only")
#doc(hidden)
pub async fn suspend_for_future_read(idx : Int, val : Int) -> Unit {
  let event = if val == -1 {
    waitable_event(idx, None)
  } else {
    let result = FutureReadResult::from(val)
    waitable_event(idx, Some(FutureRead(idx, result)))
  }
  guard event is FutureRead(i, result) && i == idx
  match result {
    Completed => return
    Cancelled => raise FutureReadError::Cancelled
  }
}

///|
#internal(wit_bindgen, "generated binding code only")
#doc(hidden)
pub fn cancel_future_read(
  owner_task : Int,
  idx : Int,
  cancel : () -> Int,
) -> Bool {
  let event = cancel_waitable_event(owner_task, idx, () => {
    FutureRead(idx, FutureReadResult::from(cancel()))
  })
  guard event is FutureRead(i, result) && i == idx
  match result {
    Completed => true
    Cancelled => false
  }
}

///|
#internal(wit_bindgen, "generated binding code only")
#doc(hidden)
pub fn cancel_stream_read(
  owner_task : Int,
  idx : Int,
  cancel : () -> Int,
) -> Int {
  let event = cancel_waitable_event(owner_task, idx, () => {
    StreamRead(idx, StreamResult::from(cancel()))
  })
  guard event is StreamRead(i, result) && i == idx
  result.progress
}

///|
#internal(wit_bindgen, "generated binding code only")
#doc(hidden)
pub fn cancel_stream_write(idx : Int, cancel : () -> Int) -> Int {
  let event = cancel_waitable_event(current_component_task_token(), idx, () => {
    StreamWrite(idx, StreamResult::from(cancel()))
  })
  guard event is StreamWrite(i, result) && i == idx
  result.progress
}

///|
#internal(wit_bindgen, "generated binding code only")
#doc(hidden)
pub async fn suspend_for_future_write_terminal(idx : Int, val : Int) -> Bool? {
  let event = if val == -1 {
    waitable_event(idx, None)
  } else {
    let result = FutureWriteResult::from(val)
    waitable_event(idx, Some(FutureWrite(idx, result)))
  }
  guard event is FutureWrite(i, result) && i == idx
  match result {
    Completed => Some(true)
    Dropped => Some(false)
    Cancelled => None
  }
}

///|
#internal(wit_bindgen, "generated binding code only")
#doc(hidden)
pub async fn suspend_for_stream_read(idx : Int, val : Int) -> (Int, Bool) {
  let event = if val == -1 {
    waitable_event(idx, None)
  } else {
    let result = StreamResult::from(val)
    waitable_event(idx, Some(StreamRead(idx, result)))
  }
  guard event is StreamRead(i, { progress, copy_result }) && i == idx
  match copy_result {
    Completed => return (progress, false)
    Dropped => return (progress, true)
    Cancelled =>
      if progress > 0 {
        return (progress, false)
      } else {
        raise StreamReadCancelled
      }
  }
}

///|
#internal(wit_bindgen, "generated binding code only")
#doc(hidden)
pub async fn suspend_for_stream_write(idx : Int, val : Int) -> (Int, Bool) {
  let event = if val == -1 {
    waitable_event(idx, None)
  } else {
    let result = StreamResult::from(val)
    waitable_event(idx, Some(StreamWrite(idx, result)))
  }
  guard event is StreamWrite(i, { progress, copy_result }) && i == idx
  match copy_result {
    Completed => return (progress, false)
    Dropped => return (progress, true)
    Cancelled => return (progress, false)
  }
}

///|
pub suberror OpCancelled {
  SubTaskCancelled(before_started~ : Bool)
  StreamReadCancelled
}

///|
pub(all) suberror FutureReadError {
  Cancelled
  Dropped
}

///|
#internal(wit_bindgen, "generated binding code only")
pub(all) suberror EndpointBusy {
  Read
  Write
}
