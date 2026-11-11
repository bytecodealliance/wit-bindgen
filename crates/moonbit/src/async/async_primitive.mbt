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
async fn[T] async_suspend(cb : ((T) -> Unit) -> Unit) -> T noraise + nocancel = "%async.suspend"

///|
fn run_async(f : async () -> Unit noraise + nocancel) = "%async.run"

///|
/// Deliver cancellation from the runtime, or resume it after boundary cleanup.
#internal(wit_bindgen, "runtime and generated binding code only")
#doc(hidden)
pub async fn[X] raise_cancellation_signal() -> X noraise = "%async.raise_cancellation_signal"

///|
async fn[X] capture_cancellation(
  f : async () -> X,
  handler~ : async () -> X nocancel,
) -> X nocancel = "%async.capture_cancellation"
