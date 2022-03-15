use std::cell::RefCell;
use std::sync::Once;
use wit_bindgen_gen_core::wit_parser::Interface;
use wit_bindgen_gen_core::Generator;
use wit_bindgen_rust::Handle;

wit_bindgen_rust::export!("demo.wit");
wit_bindgen_rust::import!("browser.wit");

struct Demo;

impl demo::Demo for Demo {}

#[derive(Default)]
pub struct Config {
    js: RefCell<wit_bindgen_gen_js::Opts>,
    c: RefCell<wit_bindgen_gen_c::Opts>,
    rust: RefCell<wit_bindgen_gen_rust_wasm::Opts>,
    wasmtime: RefCell<wit_bindgen_gen_wasmtime::Opts>,
    wasmtime_py: RefCell<wit_bindgen_gen_wasmtime_py::Opts>,
    markdown: RefCell<wit_bindgen_gen_markdown::Opts>,
    spidermonkey: RefCell<wit_bindgen_gen_spidermonkey::Opts>,
    wasmer: RefCell<wit_bindgen_gen_wasmer::Opts>,
    wasmer_py: RefCell<wit_bindgen_gen_wasmer_py::Opts>,
}

impl demo::Config for Config {
    fn new() -> Handle<Config> {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let prev_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                browser::error(&info.to_string());
                prev_hook(info);
            }));
        });

        Config::default().into()
    }

    fn render(
        &self,
        lang: demo::Lang,
        wit: String,
        import: bool,
    ) -> Result<Vec<(String, String)>, String> {
        let mut gen: Box<dyn Generator> = match lang {
            demo::Lang::Rust => Box::new(self.rust.borrow().clone().build()),
            demo::Lang::Wasmtime => Box::new(self.wasmtime.borrow().clone().build()),
            demo::Lang::WasmtimePy => Box::new(self.wasmtime_py.borrow().clone().build()),
            demo::Lang::Js => Box::new(self.js.borrow().clone().build()),
            demo::Lang::C => Box::new(self.c.borrow().clone().build()),
            demo::Lang::Markdown => Box::new(self.markdown.borrow().clone().build()),
            demo::Lang::Spidermonkey => {
                let mut opts = self.spidermonkey.borrow_mut();
                opts.import_spidermonkey = true;
                opts.js = "foo.js".into();
                let script = "throw new Error('unimplemented');";
                Box::new(opts.clone().build(script))
            }
            demo::Lang::Wasmer => Box::new(self.wasmer.borrow().clone().build()),
            demo::Lang::WasmerPy => Box::new(self.wasmer_py.borrow().clone().build()),
        };
        let iface = Interface::parse("input", &wit).map_err(|e| format!("{:?}", e))?;
        let mut files = Default::default();
        let (imports, exports) = if import {
            (vec![iface], vec![])
        } else {
            (vec![], vec![iface])
        };
        gen.generate_all(&imports, &exports, &mut files);
        Ok(files
            .iter()
            .map(|(name, contents)| {
                let contents = if contents.starts_with(b"\0asm") {
                    wasmprinter::print_bytes(contents).unwrap()
                } else {
                    String::from_utf8_lossy(&contents).into()
                };
                (name.to_string(), contents)
            })
            .collect())
    }

    fn set_rust_unchecked(&self, unchecked: bool) {
        self.rust.borrow_mut().unchecked = unchecked;
    }

    fn set_wasmtime_tracing(&self, tracing: bool) {
        self.wasmtime.borrow_mut().tracing = tracing;
    }
    fn set_wasmtime_custom_error(&self, custom_error: bool) {
        browser::log("custom error");
        self.wasmtime.borrow_mut().custom_error = custom_error;
    }
    fn set_wasmtime_async(&self, async_: demo::WasmtimeAsync) {
        use wit_bindgen_gen_wasmtime::Async;

        self.wasmtime.borrow_mut().async_ = match async_ {
            demo::WasmtimeAsync::All => Async::All,
            demo::WasmtimeAsync::None => Async::None,
            demo::WasmtimeAsync::Only(list) => Async::Only(list.into_iter().collect()),
        };
    }
    fn set_wasmer_tracing(&self, tracing: bool) {
        self.wasmer.borrow_mut().tracing = tracing;
    }
    fn set_wasmer_custom_error(&self, custom_error: bool) {
        browser::log("custom error");
        self.wasmer.borrow_mut().custom_error = custom_error;
    }
    fn set_wasmer_async(&self, async_: demo::WasmtimeAsync) {
        use wit_bindgen_gen_wasmer::Async;

        self.wasmer.borrow_mut().async_ = match async_ {
            demo::WasmtimeAsync::All => Async::All,
            demo::WasmtimeAsync::None => Async::None,
            demo::WasmtimeAsync::Only(list) => Async::Only(list.into_iter().collect()),
        };
    }
}
