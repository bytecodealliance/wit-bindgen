use crate::slab::Slab;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::sync::{Arc, Weak};
use tokio::sync::mpsc::{self, Receiver, Sender, UnboundedSender};
use tokio::task::JoinHandle;
use wasmtime::{AsContextMut, Caller, Memory, Store, StoreContextMut, Table, Trap, TypedFunc};

const MAX_EVENTS: usize = 1_000;

pub struct Async<T> {
    function_table: Table,

    /// Channel used to send messages to the main event loop where this
    /// `Async<T>` is managed.
    ///
    /// Note that this is stored in a `Weak` pointer to not hold a strong
    /// reference to it because this `Async<T>` is owned by the event loop which
    /// is otherwise terminated when the `Sender<T>` is gone, hence if this were
    /// a strong reference it would loop forever.
    ///
    /// The main strong reference to this channel is held by a generated struct
    /// that will live in user code on some other task. Other references to this
    /// sender, also weak though, will live in each imported async host function
    /// when invoked.
    sender: Weak<Sender<Message<T>>>,

    /// The list of active WebAssembly coroutines that are executing in this
    /// event loop.
    ///
    /// Note that for now the term "coroutine" here is specifically used for the
    /// interface-types notion of a coroutine and does not correspond to a
    /// literal coroutine/fiber on the host. Interface types coroutines are only
    /// implemented right now with the callback ABI, meaning there's no
    /// coroutine in the sense of "there's a suspended host stack" somewhere.
    /// Instead wasm retains all state necessary for resumption and such.
    ///
    /// This list of active coroutines will have one-per-export called and when
    /// suspended the coroutines here are all guaranteed to have pending imports
    /// they're waiting on.
    ///
    /// Note that internally `Coroutines<T>` is simply a `Slab<Coroutine<T>>`
    /// and is only structured this way to have lookups via `&CoroutineId`
    /// instead of `u32` as slabs do.
    coroutines: RefCell<Coroutines<T>>,

    events: RefCell<Slab<Event>>,
    pending_events: RefCell<Vec<(Event, u32)>>,

    /// The "currently active" coroutine.
    ///
    /// This is used to persist state in the host about what coroutine is
    /// currently active so that when an import is called we can automatically
    /// assign that import's "thread" of execution to the currently active
    /// coroutine, adding it to the right import list. This enables keeping
    /// track on the host for what imports are used where and what to cancel
    /// whenever one coroutine aborts (if at all).
    cur_wasm_coroutine: CoroutineId,

    /// The next unique ID to hand out to a coroutine.
    ///
    /// This is a monotonically increasing counter which is intended to be
    /// unique for all coroutines for the lifetime of a program. This is a
    /// generational index of sorts which prevents accidentally resuing slab
    /// indices in the `coroutines` array.
    cur_unique_id: Cell<u64>,

    receiver: Receiver<Message<T>>,
}

/// An "integer" identifier for a coroutine.
///
/// This is used to uniquely identify a logical coroutine of WebAssembly
/// execution, and internally contains the slab index it's stored at as well as
/// a unique generational ID.
#[derive(Copy, Clone)]
pub struct CoroutineId {
    slab_index: u32,
    unique_id: u64,
}

struct Coroutines<T> {
    slab: Slab<Coroutine<T>>,
}

enum Message<T> {
    Execute(Start<T>, Complete<T>, UnboundedSender<CoroutineResult>),
    RunNoCoroutine(RunStandalone<T>, UnboundedSender<CoroutineResult>),
    FinishImport(Callback<T>, CoroutineId, u32),
    Cancel(CoroutineId),
}

struct Event {
    callback: TypedFunc<(u32, u32), ()>,
    coroutine: CoroutineId,
    data: u32,
}

struct Coroutine<T> {
    /// A unique ID for this coroutine which is used to ensure that even if this
    /// coroutine's slab index is reused a `CoroutineId` uniquely points to one
    /// logical coroutine. This mostly comes up where when a coroutine exits
    /// early due to a trap we need to make sure that even if the slab slot is
    /// reused we don't accidentally use some future coroutine for lingering
    /// completion callbacks.
    unique_id: u64,

    /// A list of spawned tasks corresponding to imported host functions that
    /// this coroutine is waiting on. This list is appended to whenever an async
    /// host function is invoked and it's removed from when the host function
    /// completes (and the message gets back to the main loop).
    ///
    /// The primary purpose of this list is so that when a coroutine fails (via
    /// a trap) that all of the spawned host work for the coroutine can exit
    /// ASAP via an `abort()` signal on the `JoinHandle<T>`.
    pending_imports: Slab<JoinHandle<()>>,

    /// The number of imports or events that we're waiting on.
    pending_callbacks: usize,

    /// A callback to invoke whenever a coroutine's `async_export_done`
    /// completion callback is invoked. This is used by the host to deserialize
    /// the results from WebAssembly (possibly doing things like wasm
    /// malloc/free) and then sending the results on a channel.
    ///
    /// Typically this contains a `Sender<T>` internally within this closure
    /// which gets a message once all the wasm arguments have been successfully
    /// deserialized.
    complete: Option<Complete<T>>,

    sender: UnboundedSender<CoroutineResult>,
    cancel_task: Option<JoinHandle<()>>,
}

pub type HostFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type CoroutineResult = Result<Box<dyn Any + Send>, Trap>;
pub type Start<T> = Box<
    dyn for<'a> FnOnce(
            &'a mut StoreContextMut<'_, T>,
            u32,
        ) -> Pin<Box<dyn Future<Output = Result<(), Trap>> + Send + 'a>>
        + Send,
>;
pub type Callback<T> = Box<
    dyn for<'a> FnOnce(
            &'a mut StoreContextMut<'_, T>,
        ) -> Pin<Box<dyn Future<Output = Result<(), Trap>> + Send + 'a>>
        + Send,
>;
pub type Complete<T> = Box<
    dyn for<'a> FnOnce(
            &'a mut StoreContextMut<'_, T>,
            i32,
            wasmtime::Memory,
        ) -> Pin<Box<dyn Future<Output = CoroutineResult> + Send + 'a>>
        + Send,
>;
pub type RunStandalone<T> = Box<
    dyn for<'a> FnOnce(
            &'a mut StoreContextMut<'_, T>,
        ) -> Pin<Box<dyn Future<Output = CoroutineResult> + Send + 'a>>
        + Send,
>;

impl<T: Send + 'static> Async<T> {
    /// Spawns a new task which will manage async execution of wasm within the
    /// `store` provided.
    pub fn spawn(mut store: Store<T>, function_table: Table) -> AsyncHandle<T> {
        // This channel is the primary point of communication into the task that
        // we're going to spawn. This'll be bounded to ensure it doesn't get
        // overrun, and additionally the sender will be stored in an `Arc` to
        // ensure that the returned handle is the only owning handle and the
        // intenral weak handle held by `Async<T>` doesn't keep it alive to let
        // the task terminate gracefully.
        let (sender, receiver) = mpsc::channel(5 /* TODO: should this be configurable? */);
        let sender = Arc::new(sender);
        let mut cx = Async {
            function_table,
            sender: Arc::downgrade(&sender),
            coroutines: RefCell::new(Coroutines {
                slab: Slab::default(),
            }),
            events: Default::default(),
            pending_events: Default::default(),
            cur_wasm_coroutine: CoroutineId {
                slab_index: u32::MAX,
                unique_id: u64::MAX,
            },
            cur_unique_id: Cell::new(0),
            receiver,
        };

        tokio::spawn(async move { cx.run(&mut store).await });
        AsyncHandle { sender }
    }

    pub fn spawn_import(
        future: impl Future<Output = Callback<T>> + Send + 'static,
    ) -> Result<(), Trap> {
        Self::with(|cx| {
            let sender = cx.sender.clone();
            // Register a new pending import for the currently executing wasm
            // coroutine. This will ensure that full completion of this
            // coroutine is delayed until this import is resolved.
            let coroutine_id = cx.cur_wasm_coroutine;
            let mut coroutines = cx.coroutines.borrow_mut();
            let coroutine = coroutines
                .get_mut(&coroutine_id)
                .ok_or_else(|| Trap::new("cannot call async import from non-async export"))?;
            let pending_import_id = coroutine.pending_imports.next_id();
            coroutine.pending_callbacks += 1;

            // Note that `tokio::spawn` is used here to allow the `future` for
            // this import to execute in parallel, not just concurrently. The
            // result is re-acquired via sending a message on our internal
            // channel.
            let task = tokio::spawn(async move {
                let import_result = future.await;

                // If the main task has exited for some reason then it'll never
                // receive our result, but that's ok since we're trying to
                // complete a wasm import and if the main task isn't there to
                // receive it there's nothing else to do with this result. This
                // being an error is theoretically possible but should be rare.
                if let Some(sender) = sender.upgrade() {
                    let send_result = sender
                        .send(Message::FinishImport(
                            import_result,
                            coroutine_id,
                            pending_import_id,
                        ))
                        .await;
                    drop(send_result);
                }
            });

            let id = coroutine.pending_imports.insert(task);
            assert_eq!(id, pending_import_id);
            Ok(())
        })
    }

    /// Top level run-loop of the task which owns `Async<T>`, typically spawned
    /// as a separate task.
    async fn run(&mut self, store: &mut Store<T>) {
        while self.process_message(store).await {
            // ...continue on to the next message...
        }
    }

    async fn process_message(&mut self, store: &mut Store<T>) -> bool {
        let store = &mut store.as_context_mut();

        // The "highest priority" messages are those of pending events. These
        // are processed first before we get to the actual channel queue which
        // may block further for events.
        if let Some((event, arg)) = self.pending_events.get_mut().pop() {
            return self
                .execute_coroutine(
                    event.coroutine,
                    event.callback.call_async(store, (event.data, arg)),
                )
                .await;
        }

        // Wait for a message, but if there are no other messages then we're
        // done processing messages
        let coroutines = self.coroutines.get_mut();
        let msg = match self.receiver.recv().await {
            Some(msg) => msg,
            None => return false,
        };
        match msg {
            // This message is the start of a new task ("coroutine" in
            // interface-types-vernacular) so we allocate a new task in our
            // slab and set up its state.
            //
            // Note that we spawn a "helper" task here to send a message to
            // our channel when the `sender` specified here is disconnected.
            // That scenario means that this coroutine is cancelled and by
            // sending a message into our channel we can start processing
            // that.
            Message::Execute(run, complete, sender) => {
                let unique_id = self.cur_unique_id.get();
                self.cur_unique_id.set(unique_id + 1);
                let coroutine_id = coroutines.next_id(unique_id);
                let my_sender = self.sender.clone();
                let await_close_sender = sender.clone();
                let cancel_task = tokio::spawn(async move {
                    await_close_sender.closed().await;
                    // if the main task is gone one way or another we ignore
                    // the error here since no one's going to receive it
                    // anyway and all relevant work should be cancelled.
                    if let Some(sender) = my_sender.upgrade() {
                        drop(sender.send(Message::Cancel(coroutine_id)).await);
                    }
                });
                coroutines.insert(Coroutine {
                    unique_id,
                    complete: Some(complete),
                    sender,
                    pending_imports: Slab::default(),
                    pending_callbacks: 1,
                    cancel_task: Some(cancel_task),
                });
                self.execute_coroutine(coroutine_id, run(store, coroutine_id.slab_index))
                    .await
            }

            // This message means that we need to execute `run` specified
            // which is a "non blocking"-in-the-coroutine-sense wasm
            // function. This is basically "go run that single callback" and
            // is currently only used for things like resource destructors.
            // These aren't allowed to call blocking functions and a trap is
            // generated if they try to call a blocking function (since
            // there isn't a coroutine set up).
            //
            // Note that here we avoid allocating a coroutine entirely since
            // this isn't actually a coroutine, which means that any attempt
            // to call a blocking function will be met with failure (a
            // trap). Additionally note that the actual execution of the
            // wasm here is select'd against the closure of the `sender`
            // here as well, since if the runtime becomes disinterested in
            // the result of this async call we can interrupt and abort the
            // wasm.
            //
            // Finally note that if the wasm completes but we fail to send
            // the result of the wasm to the receiver then we ignore the
            // error since that was basically a race between wasm exiting
            // and the sender being closed.
            //
            // TODO: should this dropped result/error get logged/processed
            // somewhere?
            Message::RunNoCoroutine(run, sender) => {
                tokio::select! {
                    r = tls::scope(self, run(store)) => {
                        let is_trap = r.is_err();
                        let _ = sender.send(r);

                        // Shut down this reactor if a trap happened because
                        // the instance is now in an indeterminate state.
                        if is_trap {
                            return false;
                        }
                    }
                    _ = sender.closed() => return false,
                }
                true
            }

            // This message indicates that an import has completed and
            // the completion callback for the wasm must be executed.
            // This, plus the serialization of the arguments into wasm
            // according to the canonical ABI, is represented by
            // `run`.
            //
            // Note, though, that in some cases we don't actually run
            // the completion callback. For example if a previous
            // completion callback for this wasm task has failed with a
            // trap we don't continue to run completion callbacks for
            // the wasm task. This situation is indicated when the
            // coroutine is not actually present in our `coroutines`
            // list, so we do a lookup here before allowing execution. When
            // the coroutine isn't present we simply skip this message which
            // will run destructors for any relevant host values.
            Message::FinishImport(run, coroutine_id, import_id) => {
                let coroutine = match coroutines.get_mut(&coroutine_id) {
                    Some(c) => c,
                    None => return true,
                };
                coroutine.pending_imports.remove(import_id).unwrap();
                self.execute_coroutine(coroutine_id, run(&mut store.into()))
                    .await
            }

            // This message indicates that the specified coroutine has been
            // cancelled, meaning that the sender which would send back the
            // result of the coroutine is now a closed channel that we can
            // no longer send a message along. Our response to this is to
            // remove the coroutine, and its destructor will trigger further
            // cancellation if necessary.
            //
            // Note that this message may race with the actual completion of
            // the coroutine so we don't assert that the ID specified here
            // is actually in our list. If a coroutine is removed though we
            // assume that the wasm is now in an indeterminate state which
            // results in aborting this reactor task. If nothing is removed
            // then we assume the race was properly resolved and we skip
            // this message.
            Message::Cancel(coroutine_id) => {
                if coroutines.remove(&coroutine_id).is_some() {
                    return false;
                }
                true
            }
        }
    }

    async fn execute_coroutine(
        &mut self,
        coroutine_id: CoroutineId,
        wasm_execution: impl Future<Output = Result<(), Trap>>,
    ) -> bool {
        // Actually execute the WebAssembly callback. The call to
        // `to_execute.run` here is what will actually execute WebAssembly
        // asynchronously, and note that it's also executed within a
        // `tls::scope` to ensure that the `tls::with` function will work
        // for the duration of the future.
        //
        // Also note, though, that we want to be able to cancel the
        // execution of this WebAssembly if the caller becomes disinterested
        // in the result. This happens by using the `closed()` method on the
        // channel back to the sender, and if that happens we abort wasm
        // entirely and abort the whole coroutine by removing it later.
        //
        // If this wasm operations is aborted then we exit this loop
        // entirely and tear down this reactor task. That triggers
        // cancellation of all spawned sub-tasks and sibling coroutines, and
        // the rationale for this is that we zapped wasm while it was
        // executing so it's now in an indeterminate state and not one that
        // we can resume.
        //
        // TODO: this is a `clone()`-per-callback which is probably cheap,
        // but this is also a sort of wonky setup so this may wish to change
        // in the future.
        let coroutine = self.coroutines.get_mut().get_mut(&coroutine_id).unwrap();
        coroutine.pending_callbacks -= 1;
        let cancel_signal = coroutine.sender.clone();
        let prev_coroutine_id = mem::replace(&mut self.cur_wasm_coroutine, coroutine_id);
        let result = tokio::select! {
            r = tls::scope(self, wasm_execution) => r,
            _ = cancel_signal.closed() => return false,
        };
        self.cur_wasm_coroutine = prev_coroutine_id;

        let coroutines = self.coroutines.get_mut();
        let coroutine = coroutines.get_mut(&coroutine_id).unwrap();
        if let Err(trap) = result {
            // Our WebAssembly callback trapped. That means that this
            // entire coroutine is now in a failure state. No further
            // wasm callbacks will be invoked and the coroutine is
            // removed from out internal list to invoke the failure
            // callback, informing what trap caused the failure.
            //
            // Note that this reopens `coroutine_id.slab_index` to get
            // possibly reused, intentionally so, which is why
            // `CoroutineId` is a form of generational ID which is
            // resilient to this form of reuse. In other words when we
            // remove the result here if in the future a pending import
            // for this coroutine completes we'll simply discard the
            // message.
            //
            // Any error in sending the trap along the coroutine's channel
            // is ignored since we can race with the coroutine getting
            // dropped.
            //
            // TODO: should the trap still be sent somewhere? Is this ok to
            // simply ignore?
            //
            // Finally we exit the reactor in this case because traps
            // typically represent fatal conditions for wasm where we can't
            // really resume since it may be in an indeterminate state (wasm
            // can't handle traps itself), so after we inform the original
            // coroutine of the original trap we break out and cancel all
            // further execution.
            let coroutine = coroutines.remove(&coroutine_id).unwrap();
            let _ = coroutine.sender.send(Err(trap));
            return false;
        } else if coroutine.pending_callbacks == 0 {
            // Our wasm callback succeeded, and there are no pending
            // imports for this coroutine.
            //
            // In this state it means that the coroutine has completed
            // since no further work can possibly happen for the
            // coroutine. This means that we can safely remove it from
            // our internal list.
            //
            // If the coroutine's completion wasn't ever signaled,
            // however, then that indicates a bug in the wasm code
            // itself. This bug is translated into a trap which will get
            // reported to the caller to inform the original invocation
            // of the export that the result of the coroutine never
            // actually came about.
            //
            // Note that like above a failure to send a trap along the
            // channel is ignored since we raced with the caller becoming
            // disinterested in the result which is fine to happen at any
            // time.
            //
            // TODO: should the trap still be sent somewhere? Is this ok to
            // simply ignore?
            //
            // TODO: should this tear down the reactor as well, despite it
            // being a synthetically created trap?
            let coroutine = coroutines.remove(&coroutine_id).unwrap();
            if coroutine.complete.is_some() {
                let _ = coroutine
                    .sender
                    .send(Err(Trap::new("completion callback never called")));
            }
        } else {
            // Our wasm callback succeeded, and there are pending
            // imports for this coroutine.
            //
            // This means that the coroutine isn't finished yet so we
            // simply turn the loop and wait for something else to
            // happen. We'll next be executing WebAssembly when one of
            // the coroutine's imports finish.
        }

        true
    }

    pub async fn async_export_done(
        mut caller: Caller<'_, T>,
        task_id: i32,
        ptr: i32,
        mem: Memory,
    ) -> Result<(), Trap> {
        // Extract the completion callback registered in Rust for the `task_id`.
        // This will deserialize all of the canonical ABI results specified by
        // `ptr`, and presumably send the result on some sort of channel back to the
        // task that originally invoked the wasm.
        let task_id = task_id as u32;
        let complete = Self::with(|cx| {
            let mut coroutines = cx.coroutines.borrow_mut();
            let coroutine = coroutines
                .slab
                .get_mut(task_id)
                .ok_or_else(|| Trap::new("async context not valid"))?;
            coroutine
                .complete
                .take()
                .ok_or_else(|| Trap::new("async context not valid"))
        })?;

        // Note that this is an async-enabled call to allow `call_async` for things
        // like fuel in case the completion callback needs to invoke wasm
        // asychronously for things like deallocation.
        let result = complete(&mut caller.as_context_mut(), ptr, mem).await?;

        // With the final result of the coroutine we send this along the channel
        // back to the original task which was waiting for the result. Note that
        // this send may fail if we're racing with cancellation of this task,
        // and if cancellation happens we translate that to a trap to ensure
        // that wasm is cleaned up quickly (as oppose to waiting for the next
        // yield point where it should get cleaned up anyway).
        Self::with(|cx| {
            let mut coroutines = cx.coroutines.borrow_mut();
            let coroutine = coroutines.slab.get_mut(task_id).unwrap();
            coroutine
                .sender
                .send(Ok(result))
                .map_err(|_| Trap::new("task has been cancelled"))
        })
    }

    /// Implementation of the `event_new` canonical ABI intrinsic
    pub fn event_new(mut caller: Caller<'_, T>, cb: u32, data: u32) -> Result<u32, Trap> {
        Self::with(|cx| {
            // First up validate `cb` to ensure it's actually a valid wasm
            // callback to have given us.
            let callback = cx
                .function_table
                .get(&mut caller, cb)
                .ok_or_else(|| Trap::new("out-of bounds function index"))?
                .funcref()
                .ok_or_else(|| Trap::new("not a funcref table"))?
                .ok_or_else(|| Trap::new("callback cannot be null"))?
                .typed(&caller)?;

            // Next record that there's a pending callback because the coroutine
            // is now possibly blocked on this event.
            cx.coroutines
                .borrow_mut()
                .get_mut(&cx.cur_wasm_coroutine)
                .unwrap()
                .pending_callbacks += 1;

            // And here the event data is saved and returned back to wasm as an
            // index.
            Ok(cx.events.borrow_mut().insert(Event {
                callback,
                coroutine: cx.cur_wasm_coroutine,
                data,
            }))
        })
    }

    /// Implementation of the `event_signal` canonical ABI intrinsic
    pub fn event_signal(_caller: Caller<'_, T>, event: u32, arg: u32) -> Result<(), Trap> {
        Self::with(|cx| {
            // Validate that `event` is valid for this wasm module and then
            // enqueue the event into an intenral list to get processed after
            // this wasm is finished executing.
            //
            // Note that we don't decrement the pending imports count here but
            // rather wait for the pending event message to get processed to
            // actually decrement the count, lest the coroutine accidentally be
            // declared done early.
            let event = cx
                .events
                .borrow_mut()
                .remove(event)
                .ok_or_else(|| Trap::new("invalid event index"))?;
            let mut pending_events = cx.pending_events.borrow_mut();
            if pending_events.len() >= MAX_EVENTS {
                return Err(Trap::new("too many events created"));
            }
            pending_events.push((event, arg));
            Ok(())
        })
    }

    // TODO: this is a pretty bad interface to manage the table with...
    pub fn function_table() -> Table {
        Self::with(|cx| cx.function_table)
    }

    fn with<R>(f: impl FnOnce(&Async<T>) -> R) -> R {
        tls::with(|cx| f(cx.downcast_ref().unwrap()))
    }
}

impl<T> Coroutines<T> {
    fn next_id(&self, unique_id: u64) -> CoroutineId {
        CoroutineId {
            unique_id,
            slab_index: self.slab.next_id(),
        }
    }

    fn insert(&mut self, coroutine: Coroutine<T>) -> CoroutineId {
        let unique_id = coroutine.unique_id;
        let slab_index = self.slab.insert(coroutine);
        CoroutineId {
            unique_id,
            slab_index,
        }
    }

    fn get_mut(&mut self, id: &CoroutineId) -> Option<&mut Coroutine<T>> {
        let entry = self.slab.get_mut(id.slab_index)?;
        if entry.unique_id == id.unique_id {
            Some(entry)
        } else {
            None
        }
    }

    fn remove(&mut self, id: &CoroutineId) -> Option<Coroutine<T>> {
        let entry = self.slab.get_mut(id.slab_index)?;
        if entry.unique_id == id.unique_id {
            self.slab.remove(id.slab_index)
        } else {
            None
        }
    }
}

impl<T> Drop for Coroutine<T> {
    fn drop(&mut self) {
        // When a coroutine is removed and dropped from the internal list of
        // coroutines then we're no longer interested in any of the results for
        // any of the spawned tasks. This means we can proactively cancel
        // anything that this coroutine might be waiting on (imported functions)
        // plus the task that's used to send a message to the "main loop" on
        // cancellation.
        if let Some(task) = &self.cancel_task {
            task.abort();
        }
        for task in self.pending_imports.iter() {
            task.abort();
        }
    }
}

pub struct AsyncHandle<T> {
    sender: Arc<Sender<Message<T>>>,
}

impl<T: Send> AsyncHandle<T> {
    /// Executes a new WebAssembly in the "reactor" that this handle is
    /// connected to.
    ///
    /// This function will execute `start` as the initial callback for the
    /// asynchronous WebAssembly to be executed. This closure receives the
    /// `Store<T>` via a handle as well as the coroutine ID that's associated
    /// with this new coroutine. It's expected that this callback produces a
    /// future which represents the execution of the initial WebAssembly
    /// callback, handling all canonical ABI translations internally.
    ///
    /// The second `complete` callback is invoked when the wasm indicates that
    /// it's finished executing (the `async_export_done` intrinsic wasm
    /// import). This is expected to produce the final result of the function.
    ///
    /// This function is an `async` function which is expected to be `.await`'d.
    /// If this function's future is dropped or cancelled then the coroutine
    /// that this executes will also be dropped/cancelled. If a wasm trap
    /// happens then that will be returned here and the coroutine will be
    /// cancelled.
    ///
    /// Note that it is possible for wasm to invoke the completion callback and
    /// still trap. In situations like that the trap is returned from this
    /// function.
    pub async fn execute<U>(
        &self,
        start: impl for<'a> FnOnce(
                &'a mut StoreContextMut<'_, T>,
                u32,
            )
                -> Pin<Box<dyn Future<Output = Result<(), Trap>> + Send + 'a>>
            + Send
            + 'static,
        complete: impl for<'a> FnOnce(
                &'a mut StoreContextMut<'_, T>,
                i32,
                wasmtime::Memory,
            )
                -> Pin<Box<dyn Future<Output = Result<U, Trap>> + Send + 'a>>
            + Send
            + 'static,
    ) -> Result<U, Trap>
    where
        U: Send + 'static,
    {
        // Note that this channel should have at most 2 messages ever sent on it
        // so it's easier to deal with an unbounded channel rather than a
        // bounded channel.
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Send a request to our "reactor task" which indicates that we'd like
        // to start execution of a new WebAssembly coroutine. The start/complete
        // callbacks provided here are the implementation of the canonical ABI
        // for this particular coroutine.
        //
        // Note that failure to send here turns into a trap. This can happen
        // when the reactor task is torn down, taking the receiver with it. When
        // wasm traps this can happen, and this means that the wasm is no longer
        // present for execution so we continue to propagate traps with a new
        // synthetic trap here.
        self.sender
            .send(Message::Execute(
                Box::new(start),
                Box::new(move |store, ptr, mem| {
                    Box::pin(async move {
                        let val = complete(store, ptr, mem).await?;
                        Ok(Box::new(val) as Box<dyn Any + Send>)
                    })
                }),
                tx,
            ))
            .await
            .map_err(|_| Trap::new("wasm reactor task has gone away -- sibling trap?"))?;

        // This is a bit of a tricky dance. Once WebAssembly is requested to be
        // executed there are a number of outcomes that can happen here:
        //
        // 1. The WebAssembly coroutine could complete successfully. This means
        //    that it eventually invokes the completion callback and no traps
        //    happened. In this case the completion value is sent on the channel
        //    and then when the wasm is all finished then the sending half of
        //    the channel is destroyed.
        //
        // 2. The WebAssembly coroutine could trap before invoking its
        //    completion callback. In this scenario the first message is a trap
        //    and there will be no second message because the coroutine is
        //    destroyed after a trap.
        //
        // 3. The WebAssembly coroutine could give us a completed value
        //    successfully, but then afterwards may trap. In this situation the
        //    first message received is the completed value of the coroutine,
        //    and the second message will be the trap that occurred.
        //
        // 4. Finally a the reactor coudl get torn down because of wasm hitting
        //    a trap (leaving it in an indeterminate state) or a bug in the
        //    reactor that panicked.
        //
        // Overall this leads us to two separate `.await` calls. The first
        // `.await` receives the first message and "propagates" traps in (4)
        // assuming that the reactor is gone due to a wasm trap. This first
        // result is `Ok` in (1)/(3), and it's `Err` in the case of (2).
        //
        // The second `.await` will wait for the full completion of the
        // coroutine in (1) but then receive `None`, should immediately receive
        // `None` for (2), and will receive a trap with (3). In all situations
        // we are guaranteed that after the second message the coroutine is
        // deleted and cleaned up.
        //
        // Note that receiving `Ok` as the second message is not possible
        // because the completion callback is invoked at most once and it's only
        // invoked if no trap has happened, which means that a successful
        // completion callback is guaranteed to be the first message.
        //
        // TODO: the time that passes between the first `.await` and the second
        // `.await` is not exposed with this function's signature. This is
        // simply a bland async function that returns the result, but embedders
        // may want to process a successful result which later traps. This API
        // should probably be redesigned to accommodate this.
        let result = rx
            .recv()
            .await
            .ok_or_else(|| Trap::new("wasm reactor task has gone away -- sibling trap?"))?;
        match rx.recv().await {
            Some(Err(trap)) => Err(trap),
            Some(Ok(_)) => unreachable!(),
            None => result.map(|e| *e.downcast().ok().unwrap()),
        }
    }

    pub async fn run_no_coroutine<U>(
        &self,
        run: impl for<'a> FnOnce(
                &'a mut StoreContextMut<'_, T>,
            )
                -> Pin<Box<dyn Future<Output = Result<U, Trap>> + Send + 'a>>
            + Send
            + 'static,
    ) -> Result<U, Trap>
    where
        U: Send + 'static,
    {
        let (tx, mut rx) = mpsc::unbounded_channel();
        self.sender
            .send(Message::RunNoCoroutine(
                Box::new(move |store| {
                    Box::pin(async move {
                        let val = run(store).await?;
                        Ok(Box::new(val) as Box<dyn Any + Send>)
                    })
                }),
                tx,
            ))
            .await
            .ok()
            .expect("reactor task should be present");
        rx.recv()
            .await
            .unwrap()
            .map(|e| *e.downcast().ok().unwrap())
    }
}

mod tls {
    use std::any::Any;
    use std::cell::Cell;
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    thread_local!(static CUR: Cell<*const (dyn Any + Send)> = Cell::new(&0));

    pub async fn scope<T>(
        val: &mut (dyn Any + Send + 'static),
        future: impl Future<Output = T>,
    ) -> T {
        struct SetTls<'a, F> {
            val: &'a mut (dyn Any + Send + 'static),
            future: F,
        }

        impl<F: Future> Future for SetTls<'_, F> {
            type Output = F::Output;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
                let (val, future) = unsafe {
                    let inner = self.get_unchecked_mut();
                    (
                        Pin::new_unchecked(&mut inner.val),
                        Pin::new_unchecked(&mut inner.future),
                    )
                };

                let x: &&mut (dyn Any + Send + 'static) = val.as_ref().get_ref();
                set(&**x, || future.poll(cx))
            }
        }

        SetTls { val, future }.await
    }

    pub fn set<R>(val: &(dyn Any + Send + 'static), f: impl FnOnce() -> R) -> R {
        return CUR.with(|slot| {
            let prev = slot.replace(val);
            let _reset = Reset(slot, prev);
            f()
        });

        struct Reset<'a, T: Copy>(&'a Cell<T>, T);

        impl<T: Copy> Drop for Reset<'_, T> {
            fn drop(&mut self) {
                self.0.set(self.1);
            }
        }
    }

    pub fn with<R>(f: impl FnOnce(&(dyn Any + Send)) -> R) -> R {
        CUR.with(|slot| {
            let val = slot.get();
            unsafe { f(&*val) }
        })
    }
}
