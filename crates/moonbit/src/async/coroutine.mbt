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
priv enum SuspendResult {
  Continue
  Cancelled
}

///|
priv enum State {
  Done
  Fail(Error)
  Cancelled
  Running
  Suspend((SuspendResult) -> Unit)
}

///|
struct Coroutine {
  coro_id : Int
  lane : TaskLane
  mut state : State
  mut shielded : Bool
  mut cancelled : Bool
  mut ready : Bool
  mut spawner : ((async () -> Unit) -> Unit)?
  downstream : Set[Coroutine]
}

///|
impl Eq for Coroutine with fn equal(c1, c2) {
  c1.coro_id == c2.coro_id
}

///|
impl Hash for Coroutine with fn hash_combine(self, hasher) {
  Hash::hash_combine(self.coro_id, hasher)
}

///|
fn Coroutine::wake(self : Coroutine) -> Unit {
  if !self.ready {
    self.ready = true
    enqueue(self)
    signal_component_task(self.lane)
  }
}

///|
pub fn is_being_cancelled() -> Bool {
  let coro = current_coroutine()
  coro.cancelled && !coro.shielded
}

///|
pub async fn check_cancellation() -> Unit noraise {
  if is_being_cancelled() {
    raise_cancellation_signal()
  }
}

///|
fn Coroutine::cancel(self : Coroutine) -> Unit {
  match self.state {
    Done | Fail(_) | Cancelled => return
    Running | Suspend(_) => ()
  }
  self.cancelled = true
  if !self.shielded {
    self.wake()
  }
}

///|
async fn suspend_check_cancel() -> SuspendResult noraise {
  guard! scheduler.curr_coro is Some(coro)
  if coro.cancelled && !coro.shielded {
    return Cancelled
  }
  coro.lane.blocking += 1
  defer {
    coro.lane.blocking -= 1
  }
  async_suspend(fn(cont) {
    guard! coro.state is Running
    coro.state = Suspend(cont)
  })
}

///|
async fn suspend() -> Unit noraise {
  match suspend_check_cancel() {
    Continue => ()
    Cancelled => raise_cancellation_signal()
  }
}

///|
fn spawn_owned(
  f : async () -> Unit,
  lane : TaskLane,
  inherited_spawner : ((async () -> Unit) -> Unit)?,
) -> Coroutine {
  scheduler.coro_id += 1
  let coro = {
    state: Running,
    ready: true,
    shielded: false,
    downstream: Set([]),
    coro_id: scheduler.coro_id,
    lane,
    cancelled: false,
    spawner: inherited_spawner,
  }
  fn run(_) {
    run_async(() => {
      // Even a task cancelled before its first turn enters its body, so entry
      // defers run. Cancellation is delivered at its first suspension/check.
      try capture_cancellation(f, handler=() => coro.state = Cancelled) catch {
        err => coro.state = Fail(err)
      } noraise {
        _ if coro.state is Running => coro.state = Done
        _ => ()
      }
      for coro in coro.downstream {
        coro.wake()
      }
      coro.downstream.clear()
    })
  }

  coro.state = Suspend(run)
  enqueue(coro)
  coro
}

///|
fn spawn(f : async () -> Unit) -> Coroutine {
  match scheduler.curr_coro {
    Some(parent) => spawn_owned(f, parent.lane, parent.spawner)
    None => {
      let waitable_set = current_waitableset()
      let task_state = component_task_state(waitable_set).unwrap()
      spawn_owned(f, task_state.lane, None)
    }
  }
}

///|
fn Coroutine::unwrap(self : Coroutine) -> SuspendResult raise {
  match self.state {
    Done => Continue
    Cancelled => Cancelled
    Fail(err) => raise err
    Running | Suspend(_) => panic()
  }
}

///|
async fn Coroutine::wait(target : Coroutine) -> SuspendResult {
  guard! scheduler.curr_coro is Some(coro)
  guard! !physical_equal(coro, target)
  match target.state {
    Done => return Continue
    Cancelled => return Cancelled
    Fail(err) => raise err
    Running | Suspend(_) => ()
  }
  target.downstream.add(coro)
  defer target.downstream.remove(coro)
  suspend()
  target.unwrap()
}

///|
fn Coroutine::check_error(coro : Coroutine) -> Unit raise {
  match coro.state {
    Fail(err) => raise err
    Done | Cancelled | Running | Suspend(_) => ()
  }
}

///|
pub async fn[X] protect_from_cancel(f : async () -> X) -> X nocancel {
  guard! scheduler.curr_coro is Some(coro)
  // The shield prevents delivery; capture_cancellation expresses that invariant
  // in the effect type without changing normal error propagation.
  capture_cancellation(handler=() => panic(), () => {
    if coro.shielded {
      f()
    } else {
      coro.shielded = true
      defer {
        coro.shielded = false
      }
      f()
    }
  })
}

///|
/// Observe cancellation without clearing the task's sticky cancelled state.
/// Returns None on cancellation, Some on success, and propagates normal errors.
pub async fn[X] handle_cancellation(f : async () -> X) -> X? {
  capture_cancellation(handler=() => None, () => Some(f()))
}
