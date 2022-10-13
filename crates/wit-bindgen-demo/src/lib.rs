use anyhow::Result;
use std::sync::Once;
use wit_bindgen_core::wit_parser::Interface;
use wit_bindgen_core::{Files, Generator};

wit_bindgen_guest_rust::export!("demo.wit");
wit_bindgen_guest_rust::import!("console.wit");

struct Demo;

impl demo::Demo for Demo {
    fn render(
        lang: demo::Lang,
        wit: String,
        options: demo::Options,
    ) -> Result<Vec<(String, String)>, String> {
        init();

        let mut files = Files::default();
        render(lang, &wit, &mut files, &options).map_err(|e| format!("{:?}", e))?;

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

fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        console::log("installing panic hook");
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            console::error(&info.to_string());
            prev_hook(info);
        }));
    });
}

fn render(lang: demo::Lang, wit: &str, files: &mut Files, options: &demo::Options) -> Result<()> {
    let iface = Interface::parse("input", &wit)?;

    let mut gen_world = |mut gen: Box<dyn Generator>| {
        let (imports, exports) = if options.import {
            (vec![iface.clone()], vec![])
        } else {
            (vec![], vec![iface.clone()])
        };
        gen.generate_all(&imports, &exports, files);
    };

    match lang {
        demo::Lang::Rust => {
            let mut opts = wit_bindgen_gen_guest_rust::Opts::default();
            opts.unchecked = options.rust_unchecked;
            gen_world(Box::new(opts.build()))
        }
        demo::Lang::Java => gen_world(Box::new(
            wit_bindgen_gen_guest_teavm_java::Opts::default().build(),
        )),
        demo::Lang::Wasmtime => {
            let mut opts = wit_bindgen_gen_host_wasmtime_rust::Opts::default();
            opts.tracing = options.wasmtime_tracing;
            gen_world(Box::new(opts.build()))
        }
        demo::Lang::WasmtimePy => gen_world(Box::new(
            wit_bindgen_gen_host_wasmtime_py::Opts::default().build(),
        )),
        demo::Lang::C => gen_world(Box::new(wit_bindgen_gen_guest_c::Opts::default().build())),
        demo::Lang::Markdown => {
            gen_world(Box::new(wit_bindgen_gen_markdown::Opts::default().build()))
        }

        // JS is different from other languages at this time where it takes a
        // component as input as opposed to an `Interface`. To work with this
        // demo a dummy component is synthesized to generate bindings for. The
        // dummy core wasm module is created from the `test_helpers` support
        // this workspace already offsets, and then `wit-component` is used to
        // synthesize a component from our input interface and dummy module.
        // Finally this component is fed into the host generator which gives us
        // the files we want.
        demo::Lang::Js => {
            let (imports, interface) = if options.import {
                (vec![iface], None)
            } else {
                (Vec::new(), Some(iface))
            };
            let dummy = test_helpers::dummy_module(&imports, &[], interface.as_ref());
            let mut encoder = wit_component::ComponentEncoder::default()
                .module(&dummy)?
                .imports(imports)?;
            if let Some(iface) = interface {
                encoder = encoder.interface(iface)?;
            }
            let wasm = encoder.encode()?;
            wit_bindgen_gen_host_js::Opts::default().generate("input", &wasm, files)?;
        }
    }

    Ok(())
}
