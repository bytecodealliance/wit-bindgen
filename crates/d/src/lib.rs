use anyhow::Result;
use heck::*;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use wit_bindgen_core::{Files, Source, WorldGenerator, wit_parser::*};

#[derive(Default)]
struct D {
    world_src: Source,
    opts: Opts,

    cur_world_fqn: String,
    interfaces: HashMap<InterfaceId, InterfaceSource>,
}

#[derive(Default)]
struct InterfaceSource {
    fqn: String,
    src: Source,
    imported: bool,
    exported: bool,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
pub struct Opts {
    /// Where to place output files
    #[cfg_attr(feature = "clap", arg(skip))]
    out_dir: Option<PathBuf>,
}

impl Opts {
    pub fn build(mut self, out_dir: Option<&PathBuf>) -> Box<dyn WorldGenerator> {
        let mut r = D::default();
        self.out_dir = out_dir.cloned();
        r.opts = self.clone();
        Box::new(r)
    }
}

fn get_package_fqn(id: PackageId, resolve: &Resolve) -> String {
    let mut ns = String::new();

    let pkg = &resolve.packages[id];
    ns.push_str("wit.");
    ns.push_str(&pkg.name.namespace.to_snake_case());
    ns.push_str(".");
    ns.push_str(&pkg.name.name.to_snake_case());
    ns.push_str(".");
    let pkg_has_multiple_versions = resolve.packages.iter().any(|(_, p)| {
        p.name.namespace == pkg.name.namespace
            && p.name.name == pkg.name.name
            && p.name.version != pkg.name.version
    });
    if pkg_has_multiple_versions {
        if let Some(version) = &pkg.name.version {
            let version = version
                .to_string()
                .replace('.', "_")
                .replace('-', "_")
                .replace('+', "_");
            ns.push_str(&version);
            ns.push_str(".");
        }
    }
    ns
}

fn get_interface_fqn(
    interface_id: &WorldKey,
    cur_world_fqn: &String,
    resolve: &Resolve,
    is_export: bool,
) -> String {
    let mut ns = String::new();
    match interface_id {
        WorldKey::Name(name) => {
            ns.push_str(cur_world_fqn);
            if is_export {
                ns.push_str(".exports")
            } else {
                ns.push_str(".imports")
            }
            ns.push_str(".");
            ns.push_str(&name.to_snake_case())
        }
        WorldKey::Interface(id) => {
            let iface = &resolve.interfaces[*id];
            ns.push_str(&get_package_fqn(iface.package.unwrap(), resolve));
            ns.push_str(&iface.name.as_ref().unwrap().to_snake_case())
        }
    }
    ns
}

fn get_world_fqn(id: WorldId, resolve: &Resolve) -> String {
    let mut ns = String::new();

    let world = &resolve.worlds[id];
    ns.push_str(&get_package_fqn(world.package.unwrap(), resolve));
    ns.push_str(&world.name.to_snake_case());
    ns
}

impl D {
    fn prepare_interface_bindings(
        &self,
        id: InterfaceId,
        fqn: &String,
        cur_world_fqn: &String,
        resolve: &Resolve,
    ) -> Source {
        let mut src = Source::default();
        let interface = &resolve.interfaces[id];

        match &interface.docs.contents {
            Some(docs) => src.push_str(&format!("/++\n{docs}\n+/\n")),
            None => {}
        }

        src.push_str(&format!("module {};\n\n", fqn));

        let mut deps = BTreeSet::new();

        for dep_id in resolve.interface_direct_deps(id) {
            deps.insert(dep_id);
        }

        for dep_id in deps {
            let wrapped_dep_id = WorldKey::Interface(dep_id);
            src.push_str(&format!(
                "import {};\n",
                get_interface_fqn(&wrapped_dep_id, cur_world_fqn, resolve, false)
            ));
        }

        src.push_str("\n// Type defines\n");

        for (name, id) in &interface.types {
            src.push_str(&format!("// Define type: {name}\n"));
        }

        src
    }
}

impl WorldGenerator for D {
    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        self.cur_world_fqn = get_world_fqn(world, resolve);

        let world = &resolve.worlds[world];
        match &world.docs.contents {
            Some(docs) => self.world_src.push_str(&format!("/++\n{docs}\n+/\n")),
            None => {}
        }
        self.world_src
            .push_str(&format!("module {};\n\n", self.cur_world_fqn));

        self.world_src.push_str("// Interface imports\n");
    }

    fn import_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        let interface_src = match self.interfaces.get_mut(&id) {
            Some(src) => src,
            None => {
                let new_fqn = get_interface_fqn(name, &self.cur_world_fqn, resolve, false);
                let new_src =
                    self.prepare_interface_bindings(id, &new_fqn, &self.cur_world_fqn, resolve);

                let mut result = InterfaceSource::default();
                result.fqn = new_fqn;
                result.src = new_src;

                self.interfaces.insert(id, result);
                self.interfaces.get_mut(&id).unwrap()
            }
        };

        if interface_src.imported {
            return Ok(());
        }
        interface_src.imported = true;

        self.world_src
            .push_str(&format!("public import {}\n", &self.interfaces[&id].fqn));

        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        self.world_src.push_str(&format!("\n// Type imports\n"));
        for (name, id) in types {
            self.world_src
                .push_str(&format!("// Define type: {name}\n"));
        }
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        self.world_src.push_str(&format!("\n// Function imports\n"));
        for (name, func) in funcs {
            self.world_src
                .push_str(&format!("// Import function: {name}\n"));
        }
    }

    fn pre_export_interface(&mut self, resolve: &Resolve, files: &mut Files) -> Result<()> {
        self.world_src.push_str("\n// Interface exports\n");
        self.world_src
            .push_str("mixin template Exports(alias Impl) {\n");
        self.world_src.indent(1);

        Ok(())
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        let interface = &resolve.interfaces[id];
        let interface_src = match self.interfaces.get_mut(&id) {
            Some(src) => src,
            None => {
                let new_fqn = get_interface_fqn(name, &self.cur_world_fqn, resolve, true);
                let new_src =
                    self.prepare_interface_bindings(id, &new_fqn, &self.cur_world_fqn, resolve);

                let mut result = InterfaceSource::default();
                result.fqn = new_fqn;
                result.src = new_src;

                self.interfaces.insert(id, result);

                self.interfaces.get_mut(&id).unwrap()
            }
        };

        if interface_src.exported {
            return Ok(());
        }
        interface_src.exported = true;

        self.world_src.push_str(&format!(
            "mixin imported!\"{}\".Exports!Impl;\n",
            interface_src.fqn
        ));

        interface_src
            .src
            .push_str("\nmixin template Exports(alias Impl) {\n");
        interface_src.src.indent(1);

        interface_src
            .src
            .push_str(&format!("// Function exports\n"));
        for (name, func) in &interface.functions {
            interface_src
                .src
                .push_str(&format!("// Export function: {name}\n"));
        }

        interface_src.src.deindent(1);
        interface_src.src.push_str("}\n");

        Ok(())
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> Result<()> {
        self.world_src.push_str(&format!("\n// Function exports\n"));
        for (name, func) in funcs {
            self.world_src
                .push_str(&format!("// Export function: {name}\n"));
        }
        Ok(())
    }

    fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) -> Result<()> {
        // Close out interface exports
        self.world_src.deindent(1);
        self.world_src.push_str("}\n");

        let mut world_filepath = PathBuf::from_iter(get_world_fqn(id, resolve).split("."));
        world_filepath.push("package.d");

        files.push(
            world_filepath.to_str().unwrap(),
            self.world_src.as_str().as_bytes(),
        );

        for (_, interface_src) in &self.interfaces {
            let mut interface_filepath = PathBuf::from_iter(interface_src.fqn.split("."));
            interface_filepath.add_extension("d");

            files.push(
                interface_filepath.to_str().unwrap(),
                interface_src.src.as_bytes(),
            );
        }
        Ok(())
    }
}
