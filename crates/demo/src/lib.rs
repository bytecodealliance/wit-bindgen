use std::sync::Once;
use witx2::abi::Direction;
use witx_bindgen_gen_core::{witx2, Generator};

witx_bindgen_rust::export!("./crates/demo/demo.witx");
witx_bindgen_rust::import!("./crates/demo/browser.witx");

struct Demo;

fn demo() -> &'static Demo {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            browser::error(&info.to_string());
            prev_hook(info);
        }));
    });

    &Demo
}

impl Demo {
    fn generate<G: Generator>(
        &self,
        witx: &str,
        import: bool,
        mut gen: G,
    ) -> Result<Vec<(String, String)>, String> {
        let iface = witx2::Interface::parse("input", witx).map_err(|e| format!("{:?}", e))?;
        let mut files = Default::default();
        gen.generate(
            &iface,
            if import {
                Direction::Import
            } else {
                Direction::Export
            },
            &mut files,
        );
        Ok(files
            .iter()
            .map(|(name, contents)| (name.to_string(), String::from_utf8_lossy(&contents).into()))
            .collect())
    }
}

impl demo::Demo for Demo {
    fn render_js(&self, witx: String, import: bool) -> Result<Vec<(String, String)>, String> {
        self.generate(&witx, import, witx_bindgen_gen_js::Opts::default().build())
    }

    fn render_rust(
        &self,
        witx: String,
        import: bool,
        unchecked: bool,
    ) -> Result<Vec<(String, String)>, String> {
        let mut opts = witx_bindgen_gen_rust_wasm::Opts::default();
        opts.unchecked = unchecked;
        self.generate(&witx, import, opts.build())
    }

    fn render_wasmtime(
        &self,
        witx: String,
        import: bool,
        tracing: bool,
        async_: demo::Async,
        custom_error: bool,
    ) -> Result<Vec<(String, String)>, String> {
        use witx_bindgen_gen_wasmtime::Async;

        let mut opts = witx_bindgen_gen_wasmtime::Opts::default();
        opts.tracing = tracing;
        opts.async_ = match async_ {
            demo::Async::All => Async::All,
            demo::Async::None => Async::None,
            demo::Async::Only(list) => Async::Only(list.into_iter().collect()),
        };
        opts.custom_error = custom_error;
        self.generate(&witx, import, opts.build())
    }
}
