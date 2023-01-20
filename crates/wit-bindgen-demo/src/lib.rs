use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::Once;
use wasm_encoder::{CustomSection, Encode, Section};
use wit_bindgen_core::component::ComponentGenerator;
use wit_bindgen_core::wit_parser::{Resolve, UnresolvedPackage};
use wit_bindgen_core::{Files, WorldGenerator};

wit_bindgen_guest_rust::generate!("demo");

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
    let mut resolve = Resolve::default();
    let pkg = resolve.push(
        UnresolvedPackage::parse(Path::new("input"), &wit)?,
        &Default::default(),
    )?;
    let (_, doc) = resolve.packages[pkg].documents.iter().next().unwrap();
    let doc = &resolve.documents[*doc];
    let world = doc
        .default_world
        .ok_or_else(|| anyhow!("no `default world` specified in document"))?;

    let gen_world = |mut gen: Box<dyn WorldGenerator>, files: &mut Files| {
        gen.generate(&resolve, world, files);
    };

    // This generator takes a component as input as opposed to an `Interface`.
    // To work with this demo a dummy component is synthesized to generate
    // bindings for. The dummy core wasm module is created from the
    // `test_helpers` support this workspace already offsets, and then
    // `wit-component` is used to synthesize a component from our input
    // interface and dummy module.  Finally this component is fed into the host
    // generator which gives us the files we want.
    let gen_component = |mut gen: Box<dyn ComponentGenerator>, files: &mut Files| {
        let mut dummy = wit_component::dummy_module(&resolve, world);
        let metadata =
            wit_component::metadata::encode(&resolve, world, wit_component::StringEncoding::UTF8)?;
        let section = CustomSection {
            name: "component-type",
            data: &metadata,
        };
        dummy.push(section.id());
        section.encode(&mut dummy);
        let wasm = wit_component::ComponentEncoder::default()
            .module(&dummy)?
            .encode()?;
        wit_bindgen_core::component::generate(&mut *gen, "input", &wasm, files)
    };

    match lang {
        demo::Lang::Rust => {
            let mut opts = wit_bindgen_gen_guest_rust::Opts::default();
            opts.unchecked = options.rust_unchecked;
            gen_world(opts.build(), files)
        }
        demo::Lang::Java => gen_world(
            wit_bindgen_gen_guest_teavm_java::Opts::default().build(),
            files,
        ),
        demo::Lang::C => gen_world(wit_bindgen_gen_guest_c::Opts::default().build(), files),
        demo::Lang::Markdown => gen_world(wit_bindgen_gen_markdown::Opts::default().build(), files),
        demo::Lang::Js => {
            let mut opts = wit_bindgen_gen_host_js::Opts::default();
            opts.instantiation = options.js_instantiation;
            opts.compat = options.js_compat;
            gen_component(opts.build()?, files)?
        }
    }

    Ok(())
}
