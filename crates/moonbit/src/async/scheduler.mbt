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
priv struct Scheduler {
  mut coro_id : Int
  mut curr_coro : Coroutine?
}

///|
/// The per-component-task part of the scheduler. Coroutines retain this lane,
/// not `ComponentTaskState`, so a retired task cannot be kept alive by an RC
/// cycle through its registry entry or subscribers.
priv struct TaskLane {
  mut blocking : Int
  run_later : @deque.Deque[Coroutine]
  mut wakeup : TaskWakeup?
}

///|
let scheduler : Scheduler = { coro_id: 0, curr_coro: None }

///|
fn current_coroutine() -> Coroutine {
  scheduler.curr_coro.unwrap()
}

///|
fn enqueue(coro : Coroutine) -> Unit {
  coro.lane.run_later.push_back(coro)
}

///|
fn has_immediately_ready_task(lane : TaskLane) -> Bool {
  !lane.run_later.is_empty()
}

///|
fn no_more_work(lane : TaskLane) -> Bool {
  lane.blocking == 0 && lane.run_later.is_empty()
}

///|
/// Run one fair scheduling round for a component task. Coroutines woken or
/// spawned during this round are left for the next round so the component
/// event loop gets a chance to poll completed waitables.
fn reschedule(lane : TaskLane) -> Unit {
  let count = lane.run_later.length()
  for _ in 0..<count {
    guard lane.run_later.pop_front() is Some(coro) else { break }
    coro.ready = false
    guard coro.state is Suspend(ok_cont~, err_cont~) else {  }
    coro.state = Running
    let last_coro = scheduler.curr_coro
    scheduler.curr_coro = Some(coro)
    if coro.cancelled && !coro.shielded {
      err_cont(Cancelled::Cancelled)
    } else {
      ok_cont(())
    }
    scheduler.curr_coro = last_coro
  }
}
