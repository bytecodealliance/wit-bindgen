witx_bindgen_wasmtime::import!("./tests/runtime/async_functions/imports.witx");

use anyhow::Result;
use futures_channel::oneshot::{channel, Receiver, Sender};
use futures_util::FutureExt;
use imports::*;
use wasmtime::{Engine, Linker, Module, Store};
use witx_bindgen_wasmtime::{Async, HostFuture};

#[derive(Default)]
pub struct MyImports {
    thunk_hit: bool,
    unblock1: Option<Sender<()>>,
    unblock2: Option<Sender<()>>,
    unblock3: Option<Sender<()>>,
    wait1: Option<Receiver<()>>,
    wait2: Option<Receiver<()>>,
    wait3: Option<Receiver<()>>,
}

impl Imports for MyImports {
    fn thunk(&mut self) -> HostFuture<()> {
        self.thunk_hit = true;
        Box::pin(async {
            async {}.await;
        })
    }

    fn concurrent1(&mut self, a: u32) -> HostFuture<u32> {
        assert_eq!(a, 1);
        self.unblock1.take().unwrap().send(()).unwrap();
        let wait = self.wait1.take().unwrap();
        Box::pin(async move {
            wait.await.unwrap();
            a + 10
        })
    }

    fn concurrent2(&mut self, a: u32) -> HostFuture<u32> {
        assert_eq!(a, 2);
        self.unblock2.take().unwrap().send(()).unwrap();
        let wait = self.wait2.take().unwrap();
        Box::pin(async move {
            wait.await.unwrap();
            a + 10
        })
    }

    fn concurrent3(&mut self, a: u32) -> HostFuture<u32> {
        assert_eq!(a, 3);
        self.unblock3.take().unwrap().send(()).unwrap();
        let wait = self.wait3.take().unwrap();
        Box::pin(async move {
            wait.await.unwrap();
            a + 10
        })
    }
}

witx_bindgen_wasmtime::export!("./tests/runtime/async_functions/exports.witx");

fn run(wasm: &str) -> Result<()> {
    struct Context {
        wasi: wasmtime_wasi::WasiCtx,
        imports: MyImports,
        async_: Async<Context>,
        exports: exports::ExportsData,
    }

    let engine = Engine::default();
    let module = Module::from_file(&engine, wasm)?;
    let mut linker = Linker::<Context>::new(&engine);
    imports::add_imports_to_linker(&mut linker, |cx| (&mut cx.imports, &mut cx.async_))?;
    wasmtime_wasi::add_to_linker(&mut linker, |cx| &mut cx.wasi)?;

    let mut store = Store::new(
        &engine,
        Context {
            wasi: crate::default_wasi(),
            imports: MyImports::default(),
            async_: Default::default(),
            exports: Default::default(),
        },
    );
    let (exports, _instance) =
        exports::Exports::instantiate(&mut store, &module, &mut linker, |cx| {
            (&mut cx.exports, &mut cx.async_)
        })?;

    let import = &mut store.data_mut().imports;

    // Initialize various channels which we use as synchronization points to
    // test the concurrent aspect of async wasm. The first channels here are
    // used to wait for host functions to get entered by the wasm.
    let (a, mut wait1) = channel();
    import.unblock1 = Some(a);
    let (a, mut wait2) = channel();
    import.unblock2 = Some(a);
    let (a, mut wait3) = channel();
    import.unblock3 = Some(a);

    // This second set of channels are used to unblock host futures that wasm
    // calls, simulating work that returns back to the host and takes some time
    // to complete.
    let (unblock1, b) = channel();
    import.wait1 = Some(b);
    let (unblock2, b) = channel();
    import.wait2 = Some(b);
    let (unblock3, b) = channel();
    import.wait3 = Some(b);

    futures_executor::block_on(async {
        exports.thunk(&mut store).await?;
        assert!(store.data_mut().imports.thunk_hit);

        let mut future = Box::pin(exports.test_concurrent(&mut store)).fuse();

        // wait for all three concurrent methods to get entered. Note that we
        // poll the `future` while we're here as well to run any callbacks
        // inside as necessary, but it shouldn't ever finish.
        let mut done = 0;
        while done < 3 {
            futures_util::select! {
                _ = future => unreachable!(),
                r = wait1 => { r.unwrap(); done += 1; }
                r = wait2 => { r.unwrap(); done += 1; }
                r = wait3 => { r.unwrap(); done += 1; }
            }
        }

        // Now we can "complete" the async task that each function was waiting
        // on. Our original future shouldn't be done until they're all complete.
        unblock3.send(()).unwrap();
        futures_util::select! {
            _ = future => unreachable!(),
            default => {}
        }
        unblock2.send(()).unwrap();
        futures_util::select! {
            _ = future => unreachable!(),
            default => {}
        }
        unblock1.send(()).unwrap();
        future.await?;

        Ok(())
    })
}
