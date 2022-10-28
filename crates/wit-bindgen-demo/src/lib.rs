use anyhow::Result;
use std::sync::Once;
use wit_bindgen_core::component::ComponentGenerator;
use wit_bindgen_core::wit_parser::Interface;
use wit_bindgen_core::{Files, Generator, WorldGenerator};
use wit_component::ComponentInterfaces;

wit_bindgen_guest_rust::generate!({
    import: "console.wit",
    default: "demo.wit",
    name: "demo",
});

struct Demo;

export_demo!(Demo);

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
    let interfaces = ComponentInterfaces {
        imports: if options.import {
            [(iface.name.clone(), iface.clone())].into_iter().collect()
        } else {
            Default::default()
        },
        exports: Default::default(),
        default: if !options.import {
            Some(iface.clone())
        } else {
            None
        },
    };

    let gen_world_legacy = |mut gen: Box<dyn Generator>, files: &mut Files| {
        let (imports, exports) = if options.import {
            (vec![iface.clone()], vec![])
        } else {
            (vec![], vec![iface.clone()])
        };
        gen.generate_all(&imports, &exports, files);
    };

    let gen_world = |mut gen: Box<dyn WorldGenerator>, files: &mut Files| {
        gen.generate("demo", &interfaces, files);
    };

    // This generator takes a component as input as opposed to an `Interface`.
    // To work with this demo a dummy component is synthesized to generate
    // bindings for. The dummy core wasm module is created from the
    // `test_helpers` support this workspace already offsets, and then
    // `wit-component` is used to synthesize a component from our input
    // interface and dummy module.  Finally this component is fed into the host
    // generator which gives us the files we want.
    let gen_component = |mut gen: Box<dyn ComponentGenerator>, files: &mut Files| {
        let dummy = test_helpers::dummy_module(&interfaces);
        let wasm = wit_component::ComponentEncoder::default()
            .module(&dummy)?
            .interfaces(interfaces.clone(), wit_component::StringEncoding::UTF8)?
            .encode()?;
        wit_bindgen_core::component::generate(&mut *gen, "input", &wasm, files)
    };

    match lang {
        demo::Lang::Rust => {
            let mut opts = wit_bindgen_gen_guest_rust::Opts::default();
            opts.unchecked = options.rust_unchecked;
            gen_world(opts.build(), files)
        }
        demo::Lang::Java => gen_world_legacy(
            Box::new(wit_bindgen_gen_guest_teavm_java::Opts::default().build()),
            files,
        ),
        demo::Lang::Wasmtime => {
            let mut opts = wit_bindgen_gen_host_wasmtime_rust::Opts::default();
            opts.tracing = options.wasmtime_tracing;
            gen_world(opts.build(), files)
        }
        demo::Lang::WasmtimePy => gen_component(
            wit_bindgen_gen_host_wasmtime_py::Opts::default().build(),
            files,
        )?,
        demo::Lang::C => gen_world(wit_bindgen_gen_guest_c::Opts::default().build(), files),
        demo::Lang::Markdown => gen_world(wit_bindgen_gen_markdown::Opts::default().build(), files),
        demo::Lang::Js => gen_component(wit_bindgen_gen_host_js::Opts::default().build()?, files)?,
    }

    Ok(())
}
