use wit_bindgen_core::{Source, WorldGenerator};

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
        resolve: &wit_bindgen_core::wit_parser::Resolve,
        name: &wit_bindgen_core::wit_parser::WorldKey,
        iface: wit_bindgen_core::wit_parser::InterfaceId,
        files: &mut wit_bindgen_core::Files,
    ) {
        todo!()
    }

    fn export_interface(
        &mut self,
        resolve: &wit_bindgen_core::wit_parser::Resolve,
        name: &wit_bindgen_core::wit_parser::WorldKey,
        iface: wit_bindgen_core::wit_parser::InterfaceId,
        files: &mut wit_bindgen_core::Files,
    ) -> anyhow::Result<()> {
        todo!()
    }

    fn import_funcs(
        &mut self,
        resolve: &wit_bindgen_core::wit_parser::Resolve,
        world: wit_bindgen_core::wit_parser::WorldId,
        funcs: &[(&str, &wit_bindgen_core::wit_parser::Function)],
        files: &mut wit_bindgen_core::Files,
    ) {
        todo!()
    }

    fn export_funcs(
        &mut self,
        resolve: &wit_bindgen_core::wit_parser::Resolve,
        world: wit_bindgen_core::wit_parser::WorldId,
        funcs: &[(&str, &wit_bindgen_core::wit_parser::Function)],
        files: &mut wit_bindgen_core::Files,
    ) -> anyhow::Result<()> {
        todo!()
    }

    fn import_types(
        &mut self,
        resolve: &wit_bindgen_core::wit_parser::Resolve,
        world: wit_bindgen_core::wit_parser::WorldId,
        types: &[(&str, wit_bindgen_core::wit_parser::TypeId)],
        files: &mut wit_bindgen_core::Files,
    ) {
        todo!()
    }

    fn finish(&mut self, resolve: &wit_bindgen_core::wit_parser::Resolve, world: wit_bindgen_core::wit_parser::WorldId, files: &mut wit_bindgen_core::Files) -> anyhow::Result<()> {
        todo!()
    }
}
