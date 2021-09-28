use crate::rt::RawMem;
use crate::slab::Slab;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use wasmtime::Table;
use wasmtime::{StoreContextMut, Trap};

pub struct Async<T> {
    exports: Slab<FutureState>,

    table: Option<Table>,

    /// List of imports that we're waiting on.
    ///
    /// This is a list of async imports that have been called as part of calling
    /// wasm and are registered here. When these imports complete they produce a
    /// result which then itself produces another future. The result is given a
    /// `StoreContextMut` and is expected to further execute WebAssembly,
    /// translating the results of the async host import to wasm and then
    /// invoking the wasm completion callback. When the wasm completion callback
    /// is finished then the future is complete.
    //
    // TODO: should this be in `FutureState` because imports-called are a
    // per-export thing?
    imports: Vec<Pin<Box<dyn Future<Output = ImportResult<T>> + Send>>>,
}

impl<T> Default for Async<T> {
    fn default() -> Async<T> {
        Async {
            exports: Slab::default(),
            imports: Vec::new(),
            table: None,
        }
    }
}

struct FutureState {
    results: Vec<i64>, // TODO: shouldn't need to heap-allocate this
    done: bool,
}

pub type HostFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// The result of a host import. This is mostly synthesized by bindings and
/// represents that a host import produces a closure. The closure is given
/// context to execute WebAssembly and then the execution itself results in a
/// future. This returned future represents the completion of the WebAssembly
/// itself.
pub type ImportResult<T> = Box<
    dyn for<'a> FnOnce(
            &'a mut StoreContextMut<'_, T>,
        ) -> Pin<Box<dyn Future<Output = Result<(), Trap>> + Send + 'a>>
        + Send,
>;

impl<T> Async<T> {
    /// Implementation of the `async_export_done` canonical ABI function.
    ///
    /// The first two parameters are provided by wasm itself, and the `mem` is
    /// the wasm's linear memory. The first parameter `cx` is the original value
    /// returned by `start_async_export` and indicates which call to which
    /// export is being completed. The `ptr` is a pointer into `mem` where the
    /// encoded results are located.
    pub fn async_export_done(&mut self, cx: i32, ptr: i32, mem: &[u8]) -> Result<(), Trap> {
        let cx = cx as u32;
        let dst = self
            .exports
            .get_mut(cx)
            .ok_or_else(|| Trap::new("async context not valid"))?;
        if dst.done {
            return Err(Trap::new("async context not valid"));
        }
        dst.done = true;
        for slot in dst.results.iter_mut() {
            let ptr = (ptr as u32)
                .checked_add(8)
                .ok_or_else(|| Trap::new("pointer to async completion not valid"))?;
            *slot = mem.load(ptr as i32)?;
        }
        Ok(())
    }

    /// Registers a new future returned from an async import.
    ///
    /// This function is used when an async import is invoked by wasm. The
    /// asynchronous import is represented as a future and when the future
    /// completes it needs to call the completion callback in WebAssembly. The
    /// invocation of the completion callback is represented by the output of
    /// the future here, the `ImportResult` which is a closure that takes a
    /// store context and invokes WebAssembly (further in an async fashion).
    ///
    /// Note that this doesn't actually do anything, it simply enqueues the
    /// future internally. The future will actually be driven from the
    /// `wait_for_async_export` function below.
    pub fn register_async_import(
        &mut self,
        future: impl Future<Output = ImportResult<T>> + Send + 'static,
    ) {
        self.imports.push(Box::pin(future));
    }

    /// Blocks on the completion of an asynchronous export.
    ///
    /// This function is used to await the result of an async export. In other
    /// words this is used to wait for wasm to invoke the completion callback
    /// with the `async_cx` specified.
    ///
    /// This will "block" for one of two reasons:
    ///
    /// * First is that an async import was called and the wasm's completion
    ///   callback wasn't called yet. In this scenario this function will block
    ///   on the completion of the async import.
    ///
    /// * Second is the execution of the wasm's own import completion callback.
    ///   This execution of WebAssembly may be asynchronous due to things like
    ///   fuel context switching or similar.
    ///
    /// This function invokes WebAssembly within `cx` and will not return until
    /// the completion callback for `async_cx` is invoked. When the completion
    /// callback is invoked the results of the callback are written into
    /// `results`. The `get_state` method is used to extract an `Async<T>` from
    /// the store state within `cx`.
    ///
    /// This returns `Ok(())` when the completion callback was successfully
    /// invoked, but it may also return `Err(trap)` if a trap happens while
    /// executing a wasm completion callback for an import.
    pub async fn call_async_export(
        cx: &mut StoreContextMut<'_, T>,
        results: &mut [i64],
        get_state: &(dyn Fn(&mut T) -> &mut Async<T> + Send + Sync),
        invoke_wasm: impl for<'a> FnOnce(
            &'a mut StoreContextMut<'_, T>,
            i32,
        )
            -> Pin<Box<dyn Future<Output = Result<(), Trap>> + Send + 'a>>,
    ) -> Result<(), Trap> {
        // First register a new export happening in our slab of running
        // `exports` futures.
        //
        // NB: at this time due to take `&mut StoreContextMut` as an argument to
        // this function it means that the size of `exports` is at most one. In
        // the future this will probably take some sort of async mutex and only
        // hold the mutex when wasm is running to allow concurrent execution of
        // wasm.
        let async_cx = get_state(cx.data_mut()).exports.insert(FutureState {
            results: vec![0; results.len()],
            done: false,
        });

        // Once the registration is made we immediately construct the
        // `WaitForAsyncExport` helper struct. The destructor of this struct
        // will forcibly remove the registration we just made above to prevent
        // leaking anything if the wasm future is dropped and forgotten about.
        let waiter = WaitForAsyncExport {
            cx,
            async_cx,
            get_state,
        };

        // Now that things are set up this is the original invocation of
        // WebAssembly. This invocation is itself asynchronous hence we await
        // the result here.
        invoke_wasm(waiter.cx, async_cx as i32).await?;

        // Once we've invoked the export then it's our job to wait for the
        // `async_export_done` function to get invoked. That happens here as we
        // observer the state of `async_cx` within `state.exports`. If it's not
        // finished yet then that means that we need to wait for the next of a
        // set of futures to complete (those in `state.imports`), which is
        // deferred to the `WaitForNextFuture` helper struct.
        loop {
            let state = (waiter.get_state)(waiter.cx.data_mut());
            if state.exports.get(async_cx).unwrap().done {
                break;
            }

            let result = WaitForNextFuture { state }.await?;

            // TODO: while this is executing we're not polling the futures
            // inside of `Async<T>`, is that ok? Will this need to poll the
            // future inside the state in parallel with the async wasm here to
            // ensure that things work out as expected.
            result(waiter.cx).await?;
        }

        // If we're here then that means that the `async_export_done` function
        // was called, which means taht we can copy the results into the final
        // `results` slice.
        let future = (waiter.get_state)(waiter.cx.data_mut())
            .exports
            .get(async_cx)
            .unwrap();
        assert_eq!(results.len(), future.results.len());
        results.copy_from_slice(&future.results);
        return Ok(());

        /// This is a helper struct used to remove `async_cx` from `cx` on drop.
        ///
        /// This ensures that if any wasm returns a trap or if the future itself
        /// is entirely dropped that we properly clean things up and don't leak
        /// the export's async status and allow it to accidentally be
        /// "completed" by someone else.
        struct WaitForAsyncExport<'a, 'b, 'c, 'd, T> {
            cx: &'a mut StoreContextMut<'b, T>,
            async_cx: u32,
            get_state: &'c (dyn Fn(&mut T) -> &mut Async<T> + Send + Sync + 'd),
        }

        impl<T> Drop for WaitForAsyncExport<'_, '_, '_, '_, T> {
            fn drop(&mut self) {
                (self.get_state)(self.cx.data_mut())
                    .exports
                    .remove(self.async_cx)
                    .unwrap();
            }
        }
    }

    /// Returns the previously configured function table via `set_table`.
    //
    // TODO: this probably isn't the right interface, need to figure out a
    // better way to pass this table (and other intrinsics to the wasm instance)
    // around.
    pub fn table(&self) -> Table {
        self.table.expect("table wasn't set yet")
    }

    /// Stores a table to later get returned by `table()`.
    //
    // TODO: like `table`, this probably isn't the right interface
    pub fn set_table(&mut self, table: Table) {
        assert!(self.table.is_none(), "table already set");
        self.table = Some(table);
    }
}

struct WaitForNextFuture<'a, T> {
    state: &'a mut Async<T>,
}

impl<T> Future for WaitForNextFuture<'_, T> {
    type Output = Result<ImportResult<T>, Trap>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // If there aren't any active imports then that means that WebAssembly
        // hasn't invoked the completion callback for an export but it also
        // didn't call any imports to block on anything. That means that the
        // completion callback will never be called which is an error, so
        // simulate a trap happening and report it to the embedder.
        if self.state.imports.len() == 0 {
            return Poll::Ready(Err(Trap::new(
                "wasm isn't waiting on any imports but export completion callback wasn't called",
            )));
        }

        // If we have imports then we'll "block" this current future on one of
        // the sub-futures within `self.state.imports`. By polling at least one
        // future that means we'll get re-awakened whenever the sub-future is
        // ready and we'll check here again.
        //
        // If anything is ready we return the first item that we get. This means
        // that if any import has its result ready then we propagate the result
        // outwards which will invoke the completion callback for that import's
        // execution. If, after running the import completion callback, the
        // export completion callback still hasn't been invoked then we'll come
        // back here and look for other finished imports.
        //
        // TODO: this can theoretically exhibit quadratic behavior if the wasm
        // calls tons and tons of imports. This should use a more intelligent
        // future-polling mechanism to avoid re-polling everything we're not
        // interested in every time.
        for (i, import) in self.state.imports.iter_mut().enumerate() {
            match import.as_mut().poll(cx) {
                Poll::Ready(value) => {
                    drop(self.state.imports.swap_remove(i));
                    return Poll::Ready(Ok(value));
                }
                Poll::Pending => {}
            }
        }

        Poll::Pending
    }
}

fn _assert() {
    fn _assert_send<T: Send>(_: &T) {}

    fn _test(x: &mut StoreContextMut<'_, ()>) {
        let f = Async::<()>::call_async_export(x, &mut [], &|_| panic!(), |_, _| panic!());
        _assert_send(&f);
    }
}
