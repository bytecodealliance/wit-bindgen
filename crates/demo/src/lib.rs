use std::sync::Once;
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

impl demo::Demo for Demo {
    fn render(
        &self,
        witx: String,
        language: demo::Lang,
        import: bool,
    ) -> Result<Vec<(String, String)>, String> {
        let iface = witx2::Interface::parse("input", &witx).map_err(|e| format!("{:?}", e))?;
        let mut generator: Box<dyn Generator> = match language {
            demo::Lang::Rust => Box::new(witx_bindgen_gen_rust_wasm::Opts::default().build()),
            demo::Lang::Js => Box::new(witx_bindgen_gen_js::Opts::default().build()),
            demo::Lang::Wasmtime => Box::new(witx_bindgen_gen_wasmtime::Opts::default().build()),
        };
        let mut files = Default::default();
        generator.generate(&iface, import, &mut files);
        Ok(files
            .iter()
            .map(|(name, contents)| (name.to_string(), String::from_utf8_lossy(&contents).into()))
            .collect())
    }
}
