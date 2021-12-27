use wasmtime::*;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

// This macro generates a module called "renderer"
wit_bindgen_wasmtime::import!("../renderer.wit");

use renderer::*;

fn main() -> anyhow::Result<()> {
    log::trace!("Setting up everything.");
    let (mut store, plugin) = setup()?;

    println!("Welcome to the {} plugin!", plugin.name(&mut store)?);
    // Read input and render text
    loop {
        println!("Write a line here:");
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        println!("This is your rendered text:");
        print!("{}", plugin.render(&mut store, &line)?);
    }
}

fn setup() -> anyhow::Result<(
    Store<(WasiCtx, RendererData)>,
    Renderer<(WasiCtx, RendererData)>,
)> {
    // Setup wasmtime runtime
    let engine = Engine::default();
    let module = Module::from_file(&engine, "plugins/markdown.wasm")?;

    // Our instantiation is not trivial, as we use the `.wit` file,
    // so we need to use a Linker.
    let mut linker = Linker::new(&engine);
    // In the linker, we have to add support for Wasi, as it is used by the plugin.
    wasmtime_wasi::add_to_linker(&mut linker, |(wasi, _plugin_data)| wasi)?;
    // Create a Wasi context.
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_args()?
        .build();
    // In our case, there is no host data we want the plugin to modify,
    // so we pass nothing, represented by: `app_public::PluginData`...
    // But we also have to pass the wasi context just created.
    let mut store = Store::new(&engine, (wasi, renderer::RendererData {}));

    let (plugin, _instance) =
        Renderer::instantiate(&mut store, &module, &mut linker, |(_wasi, plugin_data)| {
            plugin_data
        })?;

    Ok((store, plugin))
}
