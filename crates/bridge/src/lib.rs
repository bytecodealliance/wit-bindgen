use std::fmt::Write;
use wit_bindgen_core::{
    abi::AbiVariant,
    uwriteln,
    wit_parser::{self, Function, Resolve, TypeOwner, WorldId, WorldKey},
    Source, WorldGenerator,
};

#[derive(Default)]
struct Bridge {
    src: Source,
    opts: Opts,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Output bridge code for webassembly micro runtime
    #[cfg_attr(feature = "clap", arg(long))]
    wamr: bool,
    /// w2c2 Instance name (derived from wasm file)
    #[cfg_attr(feature = "clap", arg(long))]
    instance: String,
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        let mut r = Bridge::default();
        r.opts = self.clone();
        Box::new(r)
    }
}

impl WorldGenerator for Bridge {
    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        let world = &resolve.worlds[world];
        let name = if self.opts.instance.is_empty() {
            world.name.clone()
        } else {
            self.opts.instance.clone()
        };
        uwriteln!(
            self.src,
            r#"
        #include <stdint.h>
        #include <stdio.h>
        #include "{name}.h"
        
        static {name}Instance* instance;
        static {name}Instance app_instance;
        
        void trap(Trap trap) {{
            abort();
        }}
        
        {name}Instance* get_app() {{
            if (!instance) {{
                {name}Instantiate(&app_instance, NULL);
                instance = &app_instance;
            }}
            return instance;
        }}
        "#
        );
    }

    fn import_interface(
        &mut self,
        resolve: &wit_parser::Resolve,
        name: &WorldKey,
        iface: wit_parser::InterfaceId,
        _files: &mut wit_bindgen_core::Files,
    ) {
        let world = match name {
            WorldKey::Name(n) => n.clone(),
            WorldKey::Interface(i) => resolve.interfaces[*i].name.clone().unwrap_or_default(),
        };
        uwriteln!(self.src, "// Import IF {world}");

        let mut gen = self.interface(resolve);
        for (_name, func) in resolve.interfaces[iface].functions.iter() {
            gen.generate_function(func, &TypeOwner::Interface(iface), AbiVariant::GuestImport);
        }
    }

    fn export_interface(
        &mut self,
        resolve: &wit_parser::Resolve,
        name: &WorldKey,
        iface: wit_parser::InterfaceId,
        _files: &mut wit_bindgen_core::Files,
    ) -> anyhow::Result<()> {
        let world = match name {
            WorldKey::Name(n) => n.clone(),
            WorldKey::Interface(i) => resolve.interfaces[*i].name.clone().unwrap_or_default(),
        };
        uwriteln!(self.src, "// Export IF {world}");

        let mut gen = self.interface(resolve);
        for (_name, func) in resolve.interfaces[iface].functions.iter() {
            gen.generate_function(func, &TypeOwner::Interface(iface), AbiVariant::GuestExport);
        }
        Ok(())
    }

    fn import_funcs(
        &mut self,
        resolve: &wit_parser::Resolve,
        worldid: wit_parser::WorldId,
        funcs: &[(&str, &wit_parser::Function)],
        _files: &mut wit_bindgen_core::Files,
    ) {
        let world = &resolve.worlds[worldid];
        uwriteln!(self.src, "// Import Funcs {}", world.name);
        let mut gen = self.interface(resolve);
        for (_name, func) in funcs.iter() {
            gen.generate_function(func, &TypeOwner::World(worldid), AbiVariant::GuestImport);
        }
    }

    fn export_funcs(
        &mut self,
        resolve: &wit_parser::Resolve,
        worldid: wit_parser::WorldId,
        funcs: &[(&str, &wit_parser::Function)],
        _files: &mut wit_bindgen_core::Files,
    ) -> anyhow::Result<()> {
        let world = &resolve.worlds[worldid];
        uwriteln!(self.src, "// Export Funcs {}", world.name);
        let mut gen = self.interface(resolve);
        for (_name, func) in funcs.iter() {
            gen.generate_function(func, &TypeOwner::World(worldid), AbiVariant::GuestExport);
        }
        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &wit_parser::Resolve,
        world: wit_parser::WorldId,
        types: &[(&str, wit_parser::TypeId)],
        files: &mut wit_bindgen_core::Files,
    ) {
        let world = &resolve.worlds[world];
        uwriteln!(self.src, "// Import Types {}", world.name);
    }

    fn finish(
        &mut self,
        resolve: &wit_parser::Resolve,
        world: wit_parser::WorldId,
        files: &mut wit_bindgen_core::Files,
    ) -> anyhow::Result<()> {
        let world = &resolve.worlds[world];
        files.push(&format!("{}_bridge.c", world.name), self.src.as_bytes());
        Ok(())
    }
}

impl Bridge {
    fn interface<'a>(&'a mut self, resolve: &'a Resolve) -> BridgeInterfaceGenerator<'a> {
        BridgeInterfaceGenerator { gen: self, resolve }
    }
}

struct BridgeInterfaceGenerator<'a> {
    gen: &'a mut Bridge,
    resolve: &'a Resolve,
}

impl<'a> BridgeInterfaceGenerator<'a> {
    fn generate_function(&mut self, func: &Function, owner: &TypeOwner, variant: AbiVariant) {
        uwriteln!(self.gen.src, "Func {} {:?}", func.name, variant);
    }
}
