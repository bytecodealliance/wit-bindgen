use anyhow::Result;
use imports::*;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::oneshot::{channel, Receiver, Sender};
use wasmtime::{Config, Engine, Linker, Module, Store, TrapCode};
use witx_bindgen_wasmtime::HostFuture;

witx_bindgen_wasmtime::export!({
    paths: ["./tests/runtime/async_functions/imports.witx"],
    async: *,
});

#[derive(Default)]
pub struct MyImports {
    unblock1: Option<Sender<()>>,
    unblock2: Option<Sender<()>>,
    unblock3: Option<Sender<()>>,
    wait1: Option<Receiver<()>>,
    wait2: Option<Receiver<()>>,
    wait3: Option<Receiver<()>>,

    concurrent1: Option<Receiver<()>>,
    concurrent2: Option<Receiver<()>>,

    iloop_close_on_drop: Option<Sender<()>>,
    iloop_entered: Option<Sender<()>>,

    import_cancelled_signal: Option<Sender<()>>,
    import_cancelled_entered: Vec<Sender<()>>,
}

#[witx_bindgen_wasmtime::async_trait]
impl Imports for MyImports {
    fn thunk(&mut self) -> HostFuture<()> {
        Box::pin(async {
            async {}.await;
        })
    }

    fn concurrent1(&mut self, a: u32) -> HostFuture<u32> {
        assert_eq!(a, 1);
        self.unblock1.take();
        let wait = self.wait1.take().unwrap();
        Box::pin(async move {
            wait.await.unwrap();
            a + 10
        })
    }

    fn concurrent2(&mut self, a: u32) -> HostFuture<u32> {
        assert_eq!(a, 2);
        self.unblock2.take();
        let wait = self.wait2.take().unwrap();
        Box::pin(async move {
            wait.await.unwrap();
            a + 10
        })
    }

    fn concurrent3(&mut self, a: u32) -> HostFuture<u32> {
        assert_eq!(a, 3);
        self.unblock3.take();
        let wait = self.wait3.take().unwrap();
        Box::pin(async move {
            wait.await.unwrap();
            a + 10
        })
    }

    fn concurrent_export_helper(&mut self, idx: u32) -> HostFuture<()> {
        let rx = if idx == 0 {
            self.concurrent1.take().unwrap()
        } else {
            self.concurrent2.take().unwrap()
        };
        Box::pin(async move {
            drop(rx.await);
        })
    }

    async fn iloop_entered(&mut self) {
        drop(self.iloop_entered.take());
    }

    fn import_to_cancel(&mut self) -> HostFuture<()> {
        let signal = self.import_cancelled_signal.take();
        drop(self.import_cancelled_entered.pop());
        Box::pin(async move {
            tokio::time::sleep(Duration::new(1_000, 0)).await;
            drop(signal);
        })
    }
}

witx_bindgen_wasmtime::import!({
    async: *,
    paths: ["./tests/runtime/async_functions/exports.witx"],
});

struct Context {
    wasi: wasmtime_wasi::WasiCtx,
    imports: MyImports,
    exports: exports::ExportsData,
}

fn run(wasm: &str) -> Result<()> {
    let mut config = Config::new();
    config.async_support(true);
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let module = Module::from_file(&engine, wasm)?;
    let mut linker = Linker::<Context>::new(&engine);
    imports::add_to_linker(&mut linker, |cx| &mut cx.imports)?;
    wasmtime_wasi::add_to_linker(&mut linker, |cx| &mut cx.wasi)?;
    exports::Exports::add_to_linker(&mut linker, |cx| &mut cx.exports)?;

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run_async(&engine, &module, &linker))
}

async fn run_async(engine: &Engine, module: &Module, linker: &Linker<Context>) -> Result<()> {
    let instantiate = |imports| async {
        let mut store = Store::new(
            &engine,
            Context {
                wasi: crate::default_wasi(),
                imports,
                exports: Default::default(),
            },
        );
        store.add_fuel(10_000)?;
        store.out_of_fuel_async_yield(u64::MAX, 10_000);
        let instance = linker.instantiate_async(&mut store, &module).await?;
        exports::Exports::new(store, &instance, |cx| &mut cx.exports)
    };

    let mut import = MyImports::default();

    // Initialize various channels which we use as synchronization points to
    // test the concurrent aspect of async wasm. The first channels here are
    // used to wait for host functions to get entered by the wasm.
    let (a, wait1) = channel();
    import.unblock1 = Some(a);
    let (a, wait2) = channel();
    import.unblock2 = Some(a);
    let (a, wait3) = channel();
    import.unblock3 = Some(a);
    let (tx, mut rx) = mpsc::channel::<()>(10);
    let tx2 = tx.clone();
    tokio::spawn(async move {
        assert!(wait1.await.is_err());
        drop(tx2);
    });
    let tx2 = tx.clone();
    tokio::spawn(async move {
        assert!(wait2.await.is_err());
        drop(tx2);
    });
    tokio::spawn(async move {
        assert!(wait3.await.is_err());
        drop(tx);
    });

    let (concurrent1, a) = channel();
    let (concurrent2, b) = channel();
    import.concurrent1 = Some(a);
    import.concurrent2 = Some(b);

    // This second set of channels are used to unblock host futures that
    // wasm calls, simulating work that returns back to the host and takes
    // some time to complete.
    let (unblock1, b) = channel();
    import.wait1 = Some(b);
    let (unblock2, b) = channel();
    import.wait2 = Some(b);
    let (unblock3, b) = channel();
    import.wait3 = Some(b);

    let exports = instantiate(import).await?;
    exports.thunk().await?;

    let future = exports.test_concurrent();
    tokio::pin!(future);

    // wait for all three concurrent methods to get entered, where once this
    // happens they'll all drop the handles to `rx`, meaning that when
    // entered we'll see the `rx` channel get closed.
    tokio::select! {
        _ = &mut future => unreachable!(),
        r = rx.recv() => assert!(r.is_none()),
    }

    // Now we can "complete" the async task that each function was waiting
    // on. Our original future shouldn't be done until they're all complete.
    unblock3.send(()).unwrap();
    unblock2.send(()).unwrap();
    unblock1.send(()).unwrap();
    future.await?;

    // Test concurrent exports can be invoked, here we call wasm
    // concurrently twice and complete the second one first, ensuring the
    // first one isn't finished, and then we complete the first and assert
    // it's done.
    let a = exports.concurrent_export(0);
    tokio::pin!(a);
    let b = exports.concurrent_export(1);
    drop(concurrent2);
    tokio::select! {
        r = &mut a => panic!("got result {:?}", r),
        r = b => r.unwrap(),
    }
    drop(concurrent1);
    a.await.unwrap();

    // Cancelling an infinite loop drops the reactor and the reactor doesn't
    // execute forever. This will only work if `tx`, owned by the reactor, will
    // get dropped when we cancel execution of the infinite loop.
    let (tx, rx) = channel();
    let (tx2, rx2) = channel();
    let mut imports = MyImports::default();
    imports.iloop_close_on_drop = Some(tx);
    imports.iloop_entered = Some(tx2);
    let exports = instantiate(imports).await?;
    {
        let iloop = exports.infinite_loop();
        tokio::pin!(iloop);
        // execute the iloop long enough to get into wasm and we'll get the
        // signal when the `rx2` channel is closed.
        tokio::select! {
            _ = &mut iloop => unreachable!(),
            r = rx2 => assert!(r.is_err()),
        }
    }
    assert!(rx.await.is_err());
    drop(exports);

    // Same as above, but an infinite loop in an async exported wasm function
    let (tx, rx) = channel();
    let (tx2, rx2) = channel();
    let mut imports = MyImports::default();
    imports.iloop_close_on_drop = Some(tx);
    imports.iloop_entered = Some(tx2);
    let exports = instantiate(imports).await?;
    {
        let iloop = exports.infinite_loop_async();
        tokio::pin!(iloop);
        // execute the iloop long enough to get into wasm and we'll get the
        // signal when the `rx2` channel is closed.
        tokio::select! {
            _ = &mut iloop => unreachable!(),
            r = rx2 => assert!(r.is_err()),
        }
    }
    assert!(rx.await.is_err());
    drop(exports);

    // A trap from WebAssembly should result in cancelling all imported tasks.
    // execute forever. This will only work if `tx`, owned by the reactor, will
    // get dropped when we cancel execution of the infinite loop.
    let (tx, rx) = channel();
    let mut imports = MyImports::default();
    imports.import_cancelled_signal = Some(tx);
    let trap = instantiate(imports)
        .await?
        .call_import_then_trap()
        .await
        .unwrap_err();
    assert!(
        trap.trap_code() == Some(TrapCode::UnreachableCodeReached),
        "bad error: {}",
        trap
    );
    assert!(rx.await.is_err());

    // Dropping the owned version of the bindings should recursively tear down
    // the reactor task since it's got nothing else to do at that point.
    let (tx, rx) = channel();
    let mut imports = MyImports::default();
    imports.import_cancelled_signal = Some(tx);
    instantiate(imports).await?;
    assert!(rx.await.is_err());

    // Cancelling (dropping) the outer task transitively tears down the reactor
    // and cancels imported tasks.
    let (tx, rx) = channel();
    let (tx2, rx2) = channel();
    let mut imports = MyImports::default();
    imports.import_cancelled_signal = Some(tx);
    imports.import_cancelled_entered.push(tx2);
    let exports = instantiate(imports).await?;
    {
        let f = exports.call_infinite_import();
        tokio::pin!(f);
        // execute the wasm long enough to get into it and we'll get the
        // signal when the `rx2` channel is closed.
        tokio::select! {
            _ = &mut f => unreachable!(),
            r = rx2 => assert!(r.is_err()),
        }
    }
    assert!(rx.await.is_err());
    drop(exports);

    // With multiple concurrent exports if one of them is cancelled then they
    // all get cancelled.
    let (tx, rx) = channel();
    let (tx2, rx2) = channel();
    let mut imports = MyImports::default();
    imports.import_cancelled_entered.push(tx);
    imports.import_cancelled_entered.push(tx2);
    let exports = instantiate(imports).await?;
    let a = exports.call_infinite_import();
    let b = exports.call_infinite_import();
    {
        tokio::pin!(a);
        {
            tokio::pin!(b);
            // Run this select twice to ensure both futures get into the import within wasm.
            tokio::select! {
                _ = &mut a => unreachable!(),
                _ = &mut b => unreachable!(),
                r = rx2 => assert!(r.is_err()),
            }
            tokio::select! {
                _ = &mut a => unreachable!(),
                _ = &mut b => unreachable!(),
                r = rx => assert!(r.is_err()),
            }
            // ... `b` is now dropped here
        }
        let err = a.await.unwrap_err();
        assert!(
            err.to_string().contains("wasm reactor task has gone away"),
            "bad error: {}",
            err
        );
    }
    drop(exports);

    Ok(())
}
