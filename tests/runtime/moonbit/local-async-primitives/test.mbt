//@ wasmtime-flags = '-Wcomponent-model-async'
//@ [lang]
//@ path = 'gen/interface/test/moonbit-local-async-primitives/operations/stub.mbt'

///|
suberror ExpectedPromiseFailure derive(Debug)

///|
priv struct LocalOnlyValue {
  marker : Int
}

///|
pub async fn exercise(background_group : @async-core.TaskGroup[Unit]) -> Bool {
  let semaphore = @async-core.Semaphore(1, initial_value=1)
  guard semaphore.try_acquire() else { return false }
  guard !semaphore.try_acquire() else { return false }
  semaphore.release()
  semaphore.acquire()

  let mutex = @async-core.Mutex()
  guard mutex.try_acquire() else { return false }
  guard !mutex.try_acquire() else { return false }
  let (mutex_started, mutex_started_promise) = @async-core.Future::new()
  let mutex_waiter = background_group.spawn(
    async fn() -> Bool {
      guard mutex_started_promise.complete(()) else { return false }
      mutex.acquire()
      mutex.release()
      true
    },
    allow_failure=true,
  )
  mutex_started.get()
  mutex.release()
  guard mutex_waiter.wait() else { return false }

  let condition = @async-core.CondVar()
  let condition_ready = Ref(false)
  let (condition_started, condition_started_promise) = @async-core.Future::new()
  let condition_waiter = background_group.spawn(
    async fn() -> Bool {
      guard condition_started_promise.complete(()) else { return false }
      while !condition_ready.val {
        condition.wait()
      }
      true
    },
    allow_failure=true,
  )
  condition_started.get()
  condition_ready.val = true
  condition.signal()
  condition_waiter.cancel()
  guard condition_waiter.wait() else { return false }

  let cancellation_condition = @async-core.CondVar()
  let (condition_cancel_started, condition_cancel_started_promise) = @async-core.Future::new()
  let cancelled_condition_waiter = background_group.spawn(
    async fn() -> Unit {
      guard condition_cancel_started_promise.complete(()) else { panic() }
      cancellation_condition.wait()
    },
    allow_failure=true,
  )
  condition_cancel_started.get()
  cancelled_condition_waiter.cancel()
  let condition_wait_was_cancelled = try
    cancelled_condition_waiter.wait()
  catch {
    @async-core.Cancelled::Cancelled => true
    _ => false
  } noraise {
    _ => false
  }
  guard condition_wait_was_cancelled else { return false }

  let (condition_survivor_started, condition_survivor_started_promise) = @async-core.Future::new()
  let condition_survivor = background_group.spawn(
    async fn() -> Bool {
      guard condition_survivor_started_promise.complete(()) else {
        return false
      }
      cancellation_condition.wait()
      true
    },
    allow_failure=true,
  )
  condition_survivor_started.get()
  cancellation_condition.broadcast()
  guard condition_survivor.wait() else { return false }

  let (ready_future, ready_promise) = @async-core.Future::new()
  guard ready_promise.complete(41) else { return false }
  guard ready_future.get() == 41 else { return false }

  let (local_future, local_promise) = @async-core.Future::new()
  guard local_promise.complete({ marker: 47 }) else { return false }
  guard local_future.get().marker == 47 else { return false }

  let cleaned = Ref(0)
  let (discarded_future, discarded_promise) = @async-core.Future::new_with_cleanup(fn(
      value : Int,
    ) {
      cleaned.val = value
    },
  )
  guard discarded_promise.complete(42) else { return false }
  discarded_future.drop()
  discarded_future.drop()
  guard cleaned.val == 42 else { return false }

  let (dropped_future, dropped_promise) : (
    @async-core.Future[Int],
    @async-core.Promise[Int],
  ) = @async-core.Future::new()
  dropped_future.drop()
  guard !dropped_promise.complete(43) else { return false }

  let (failed_future, failed_promise) : (
    @async-core.Future[Int],
    @async-core.Promise[Int],
  ) = @async-core.Future::new()
  guard failed_promise.fail(ExpectedPromiseFailure) else { return false }
  let observed_failure = try failed_future.get() catch {
    ExpectedPromiseFailure::ExpectedPromiseFailure => true
    _ => false
  } noraise {
    _ => false
  }
  guard observed_failure else { return false }

  let (closed_future, closed_promise) : (
    @async-core.Future[Int],
    @async-core.Promise[Int],
  ) = @async-core.Future::new()
  guard closed_promise.close() else { return false }
  let observed_close = try closed_future.get() catch {
    @async-core.PromiseClosed::PromiseClosed => true
    _ => false
  } noraise {
    _ => false
  }
  guard observed_close else { return false }

  let (pending_future, pending_promise) = @async-core.Future::new()
  let (pending_started, pending_started_promise) = @async-core.Future::new()
  let pending_waiter = background_group.spawn(
    async fn() -> Int {
      guard pending_started_promise.complete(()) else { panic() }
      pending_future.get()
    },
    allow_failure=true,
  )
  pending_started.get()
  guard pending_promise.complete(44) else { return false }
  guard pending_waiter.wait() == 44 else { return false }

  let (cancelled_future, cancelled_promise) = @async-core.Future::new()
  let (cancel_started, cancel_started_promise) = @async-core.Future::new()
  let cancelled_waiter = background_group.spawn(
    async fn() -> Int {
      guard cancel_started_promise.complete(()) else { panic() }
      cancelled_future.get()
    },
    allow_failure=true,
  )
  cancel_started.get()
  cancelled_waiter.cancel()
  let observed_cancel = try cancelled_waiter.wait() catch {
    @async-core.Cancelled::Cancelled => true
    _ => false
  } noraise {
    _ => false
  }
  guard observed_cancel else { return false }
  guard !cancelled_promise.complete(45) else { return false }

  let (dropped_while_reading, dropped_while_reading_promise) = @async-core.Future::new()
  let (drop_read_started, drop_read_started_promise) = @async-core.Future::new()
  let dropped_reader = background_group.spawn(
    async fn() -> Bool {
      guard drop_read_started_promise.complete(()) else { panic() }
      dropped_while_reading.get() catch {
        @async-core.Cancelled::Cancelled => return true
        _ => return false
      }
      false
    },
    allow_failure=true,
  )
  drop_read_started.get()
  dropped_while_reading.drop()
  guard dropped_reader.wait() else { return false }
  guard !dropped_while_reading_promise.complete(()) else { return false }

  let completion_cleanup = Ref(0)
  let (completed_while_reading, completed_while_reading_promise) = @async-core.Future::new_with_cleanup(fn(
      value : Int,
    ) {
      completion_cleanup.val = value
    },
  )
  let (completed_read_started, completed_read_started_promise) = @async-core.Future::new()
  let completed_reader = background_group.spawn(
    async fn() -> Int {
      guard completed_read_started_promise.complete(()) else { panic() }
      completed_while_reading.get()
    },
    allow_failure=true,
  )
  completed_read_started.get()
  guard completed_while_reading_promise.complete(45) else { return false }
  completed_while_reading.drop()
  guard completed_reader.wait() == 45 else { return false }
  guard completion_cleanup.val == 0 else { return false }

  let (racing_future, racing_promise) = @async-core.Future::new()
  let (race_started, race_started_promise) = @async-core.Future::new()
  let racing_waiter = background_group.spawn(
    async fn() -> Int {
      guard race_started_promise.complete(()) else { panic() }
      racing_future.get()
    },
    allow_failure=true,
  )
  race_started.get()
  guard racing_promise.complete(46) else { return false }
  racing_waiter.cancel()
  guard racing_waiter.wait() == 46 else { return false }

  let fifo = @async-core.Semaphore(1, initial_value=0)
  let phase = Ref(0)
  let (first_started, first_started_promise) = @async-core.Future::new()
  let first = background_group.spawn(
    async fn() -> Bool {
      guard first_started_promise.complete(()) else { return false }
      fifo.acquire()
      guard phase.val == 0 else { return false }
      phase.val = 1
      fifo.release()
      true
    },
    allow_failure=true,
  )
  first_started.get()
  let (second_started, second_started_promise) = @async-core.Future::new()
  let second = background_group.spawn(
    async fn() -> Bool {
      guard second_started_promise.complete(()) else { return false }
      fifo.acquire()
      guard phase.val == 1 else { return false }
      phase.val = 2
      true
    },
    allow_failure=true,
  )
  second_started.get()
  fifo.release()
  guard first.wait() else { return false }
  guard second.wait() else { return false }
  guard phase.val == 2 else { return false }

  let cancellable = @async-core.Semaphore(1, initial_value=0)
  let (cancelled_acquire_started, cancelled_acquire_started_promise) = @async-core.Future::new()
  let cancelled_acquire = background_group.spawn(
    async fn() -> Bool {
      guard cancelled_acquire_started_promise.complete(()) else { return false }
      cancellable.acquire()
      false
    },
    allow_failure=true,
  )
  cancelled_acquire_started.get()
  cancelled_acquire.cancel()
  let acquire_was_cancelled = try cancelled_acquire.wait() catch {
    @async-core.Cancelled::Cancelled => true
    _ => false
  } noraise {
    _ => false
  }
  guard acquire_was_cancelled else { return false }

  let (survivor_started, survivor_started_promise) = @async-core.Future::new()
  let survivor = background_group.spawn(
    async fn() -> Bool {
      guard survivor_started_promise.complete(()) else { return false }
      cancellable.acquire()
      true
    },
    allow_failure=true,
  )
  survivor_started.get()
  cancellable.release()
  guard survivor.wait() else { return false }

  let assigned = @async-core.Semaphore(1, initial_value=0)
  let (assigned_started, assigned_started_promise) = @async-core.Future::new()
  let assigned_waiter = background_group.spawn(
    async fn() -> Bool {
      guard assigned_started_promise.complete(()) else { return false }
      assigned.acquire()
      true
    },
    allow_failure=true,
  )
  assigned_started.get()
  assigned.release()
  assigned_waiter.cancel()
  assigned_waiter.wait()
}
