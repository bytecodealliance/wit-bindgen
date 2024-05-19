use std::fmt::Write;
use wit_bindgen_core::{
    uwriteln,
    wit_parser::{self, WorldKey},
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
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        let mut r = Bridge::default();
        r.opts = self.clone();
        Box::new(r)
    }
}

impl WorldGenerator for Bridge {
    fn import_interface(
        &mut self,
        resolve: &wit_parser::Resolve,
        name: &WorldKey,
        iface: wit_parser::InterfaceId,
        files: &mut wit_bindgen_core::Files,
    ) {
        let world = match name {
            WorldKey::Name(n) => n.clone(),
            WorldKey::Interface(i) => resolve.interfaces[*i].name.clone().unwrap_or_default(),
        };
        uwriteln!(self.src, "Import IF {world}");
    }

    fn export_interface(
        &mut self,
        resolve: &wit_parser::Resolve,
        name: &WorldKey,
        iface: wit_parser::InterfaceId,
        files: &mut wit_bindgen_core::Files,
    ) -> anyhow::Result<()> {
        let world = match name {
            WorldKey::Name(n) => n.clone(),
            WorldKey::Interface(i) => resolve.interfaces[*i].name.clone().unwrap_or_default(),
        };
        uwriteln!(self.src, "Export IF {world}");
        Ok(())
    }

    fn import_funcs(
        &mut self,
        resolve: &wit_parser::Resolve,
        world: wit_parser::WorldId,
        funcs: &[(&str, &wit_parser::Function)],
        files: &mut wit_bindgen_core::Files,
    ) {
        let world = &resolve.worlds[world];
        uwriteln!(self.src, "Import Funcs {}", world.name);
    }

    fn export_funcs(
        &mut self,
        resolve: &wit_parser::Resolve,
        world: wit_parser::WorldId,
        funcs: &[(&str, &wit_parser::Function)],
        files: &mut wit_bindgen_core::Files,
    ) -> anyhow::Result<()> {
        let world = &resolve.worlds[world];
        uwriteln!(self.src, "Export Funcs {}", world.name);
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
        uwriteln!(self.src, "Import Types {}", world.name);
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
