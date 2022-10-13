use std::sync::Once;
use wit_bindgen_core::wit_parser::Interface;
use wit_bindgen_core::Generator;

wit_bindgen_guest_rust::export!("demo.wit");
wit_bindgen_guest_rust::import!("browser.wit");

struct Demo;

impl demo::Demo for Demo {
    fn render(
        lang: demo::Lang,
        wit: String,
        options: demo::Options,
    ) -> Result<Vec<(String, String)>, String> {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let prev_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                browser::error(&info.to_string());
                prev_hook(info);
            }));
        });

        let mut gen: Box<dyn Generator> = match lang {
            demo::Lang::Rust => Box::new({
                let mut opts = wit_bindgen_gen_guest_rust::Opts::default();
                opts.unchecked = options.rust_unchecked;
                opts.build()
            }),
            demo::Lang::Java => Box::new(wit_bindgen_gen_guest_teavm_java::Opts::default().build()),
            demo::Lang::Wasmtime => Box::new({
                let mut opts = wit_bindgen_gen_host_wasmtime_rust::Opts::default();
                opts.tracing = options.wasmtime_tracing;
                opts.build()
            }),
            demo::Lang::WasmtimePy => {
                Box::new(wit_bindgen_gen_host_wasmtime_py::Opts::default().build())
            }
            demo::Lang::Js => Box::new(wit_bindgen_gen_host_js::Opts::default().build()),
            demo::Lang::C => Box::new(wit_bindgen_gen_guest_c::Opts::default().build()),
            demo::Lang::Markdown => Box::new(wit_bindgen_gen_markdown::Opts::default().build()),
        };
        let iface = Interface::parse("input", &wit).map_err(|e| format!("{:?}", e))?;
        let mut files = Default::default();
        let (imports, exports) = if options.import {
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
}
