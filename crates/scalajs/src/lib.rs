pub mod context;
pub mod interface;
pub mod jco;
pub mod resource;
pub mod rt;
pub mod world;

use crate::context::{ScalaJsContext, ScalaJsFile, ScalaKeywords};
use crate::interface::ScalaJsInterface;
use crate::rt::render_runtime_module;
use crate::world::ScalaJsWorld;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::str::FromStr;
use wit_bindgen_core::wit_parser::{Function, InterfaceId, Resolve, TypeId, WorldId, WorldKey};
use wit_bindgen_core::Direction::{Export, Import};
use wit_bindgen_core::{Files, WorldGenerator};

#[derive(Debug, Clone)]
pub enum ScalaDialect {
    Scala2,
    Scala3,
}

impl Default for ScalaDialect {
    fn default() -> Self {
        ScalaDialect::Scala2
    }
}

impl Display for ScalaDialect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScalaDialect::Scala2 => write!(f, "scala2"),
            ScalaDialect::Scala3 => write!(f, "scala3"),
        }
    }
}

impl FromStr for ScalaDialect {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "scala2" => Ok(ScalaDialect::Scala2),
            "scala3" => Ok(ScalaDialect::Scala3),
            _ => Err("Invalid Scala dialect".to_string()),
        }
    }
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    #[cfg_attr(
        feature = "clap",
        clap(long, help = "Base package for generated Scala.js code")
    )]
    pub base_package: Option<String>,

    #[cfg_attr(
        feature = "clap",
        clap(long, help = "Base package for generated Scala.js skeleton code")
    )]
    pub skeleton_base_package: Option<String>,

    #[cfg_attr(
        feature = "clap",
        clap(
            long,
            help = "Scala dialect to generate code for",
            default_value = "scala2"
        )
    )]
    pub scala_dialect: ScalaDialect,
    #[cfg_attr(
        feature = "clap",
        clap(
            long,
            help = "Generate a skeleton for implementing all the exports",
            default_value = "scala2"
        )
    )]
    pub generate_skeleton: bool,
    #[cfg_attr(
        feature = "clap",
        clap(
            long,
            help = "Relative root directory for placing the skeleton sources",
            default_value = "scala2"
        )
    )]
    pub skeleton_root: Option<String>,
    #[cfg_attr(
        feature = "clap",
        clap(
            long,
            help = "Relative root directory for placing the binding sources",
            default_value = "scala2"
        )
    )]
    pub binding_root: Option<String>,
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        Box::new(ScalaJs::new(self.clone()))
    }

    pub fn base_package_segments(&self) -> Vec<String> {
        self.base_package
            .clone()
            .map(|pkg| pkg.split('.').map(|s| s.to_string()).collect::<Vec<_>>())
            .unwrap_or_default()
    }

    pub fn base_package_prefix(&self) -> String {
        match &self.base_package {
            Some(pkg) => format!("{pkg}."),
            None => "".to_string(),
        }
    }
}

pub struct ScalaJs {
    context: ScalaJsContext,
    generated_files: Vec<ScalaJsFile>,
    world_defs: HashMap<WorldId, ScalaJsWorld>,
}

impl WorldGenerator for ScalaJs {
    fn import_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        iface: InterfaceId,
        _files: &mut Files,
    ) -> anyhow::Result<()> {
        let key = name;
        let wit_name = resolve.name_world_key(key);

        self.context.imports.insert(iface);
        let mut scalajs_iface =
            ScalaJsInterface::new(wit_name.clone(), resolve, iface, Import, self);
        scalajs_iface.generate();

        let file = scalajs_iface.finalize();
        self.generated_files.push(file);

        Ok(())
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        iface: InterfaceId,
        _files: &mut Files,
    ) -> anyhow::Result<()> {
        let key = name;
        let wit_name = resolve.name_world_key(key);

        self.context.exports.insert(iface);
        let mut scalajs_iface =
            ScalaJsInterface::new(wit_name.clone(), resolve, iface, Export, self);
        scalajs_iface.generate();

        let file = scalajs_iface.finalize();
        self.generated_files.push(file);

        Ok(())
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world_id: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let world = &resolve.worlds[world_id];

        if !self.world_defs.contains_key(&world_id) {
            let world_def = ScalaJsWorld::new(&self.context, resolve, world_id, world);
            self.world_defs.insert(world_id, world_def);
        }

        for (func_name, func) in funcs {
            self.world_defs
                .get_mut(&world_id)
                .unwrap()
                .add_imported_function(&self.context, resolve, func_name, func);
        }
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world_id: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> anyhow::Result<()> {
        let world = &resolve.worlds[world_id];

        if !self.world_defs.contains_key(&world_id) {
            let world_def = ScalaJsWorld::new(&self.context, resolve, world_id, world);
            self.world_defs.insert(world_id, world_def);
        }

        for (func_name, func) in funcs {
            self.world_defs
                .get_mut(&world_id)
                .unwrap()
                .add_exported_function(&self.context, resolve, func_name, func);
        }

        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        world_id: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let world = &resolve.worlds[world_id];

        if !self.world_defs.contains_key(&world_id) {
            let world_def = ScalaJsWorld::new(&self.context, resolve, world_id, world);
            self.world_defs.insert(world_id, world_def);
        }

        for (type_name, type_id) in types {
            self.world_defs.get_mut(&world_id).unwrap().add_type(
                &self.context,
                resolve,
                type_name,
                type_id,
            );
        }
    }

    fn finish(
        &mut self,
        resolve: &Resolve,
        _world: WorldId,
        files: &mut Files,
    ) -> anyhow::Result<()> {
        for file in &self.generated_files {
            files.push(
                &file.path(&self.context.opts.binding_root),
                file.source.as_bytes(),
            );
        }

        for (_, world_def) in self.world_defs.drain() {
            let world_files = world_def.finalize(&self.context, resolve);
            for world_file in world_files {
                files.push(
                    &world_file.path(&self.context.opts.binding_root),
                    world_file.source.as_bytes(),
                );
            }
        }

        let rt = render_runtime_module(&self.context.opts);
        files.push(
            &rt.path(&self.context.opts.binding_root),
            rt.source.as_bytes(),
        );

        Ok(())
    }
}

impl ScalaJs {
    fn new(opts: Opts) -> Self {
        let keywords = ScalaKeywords::new(&opts.scala_dialect);
        Self {
            context: ScalaJsContext {
                opts,
                keywords,
                overrides: HashMap::new(),
                imports: HashSet::new(),
                exports: HashSet::new(),
            },
            generated_files: Vec::new(),
            world_defs: HashMap::new(),
        }
    }
}
