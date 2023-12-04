use std::{collections::HashMap, mem};

use crate::{
    interface::{self, InterfaceGenerator},
    C_GEN_FILES_PATH,
};
use heck::{ToSnakeCase, ToUpperCamelCase};
use wit_bindgen_core::{
    wit_parser::{Resolve, WorldId},
    Direction, Files, Source,
};

/// Bookkeeping for the name of the world being generated and its ID.
#[derive(Default, Debug, Clone)]
pub struct TinyGoWorld {
    name: String,
    id: Option<WorldId>,
}

impl TinyGoWorld {
    pub fn from_world_id(id: WorldId, resolve: &Resolve) -> Self {
        Self {
            name: resolve.worlds[id].name.to_string(),
            id: Some(id),
        }
    }
    pub fn to_snake_case(&self) -> String {
        self.name.to_snake_case()
    }
    pub fn to_upper_camel_case(&self) -> String {
        self.name.to_upper_camel_case()
    }
    pub fn unwrap_id(&self) -> WorldId {
        self.id.unwrap()
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Bookkeeping for the name of the package being generated
/// and each package corresponds to either an imported or exported interface
/// , or all imported functions and types from the world, or all
/// exported functions from the world.
#[derive(Default)]
pub struct Packages {
    pub prefix_name: Option<String>,
    packages: HashMap<String, Package>,
}

pub struct Package {
    pub src: Source,
    pub preamble: Source,
    pub world: TinyGoWorld,
    pub path: Vec<String>,
    pub c_files_path: String,
    pub prefix_name: Option<String>,
}

impl Packages {
    pub fn push(
        &mut self,
        name: &str,
        src: Source,
        preamble: Source,
        world: &'_ TinyGoWorld,
        module_path: Vec<String>,
    ) -> Option<Package> {
        let prefix_name = self.prefix_name.clone();
        let c_files_path = get_c_files_path(&module_path);
        let mut package = Package::new(
            src,
            preamble,
            world.clone(),
            module_path,
            c_files_path,
            prefix_name,
        );

        let old_package = self.packages.insert(name.to_owned(), package);
        assert!(old_package.is_none());
        old_package
    }

    pub fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) {
        for (_, mut package) in self {
            package.finish(resolve, id);
            let mut path_name = package.path.join("/");
            path_name.push_str(".wit.go");
            files.push(&path_name, package.src.as_bytes());
        }
    }
}

fn get_c_files_path(module_path: &[String]) -> String {
    // for each path in module_path, we will go to the parent directory
    // and then join the path with the C_GEN_FILES_PATH
    let mut path = String::new();
    for _ in 0..module_path.len() - 1 {
        path.push_str("../");
    }
    path.push_str(C_GEN_FILES_PATH);
    path
}

impl Package {
    pub fn new(
        src: Source,
        preamble: Source,
        world: TinyGoWorld,
        path: Vec<String>,
        c_files_path: String,
        prefix_name: Option<String>,
    ) -> Self {
        Self {
            src,
            preamble,
            world,
            path,
            c_files_path,
            prefix_name,
        }
    }

    pub fn finish(&mut self, resolve: &Resolve, id: WorldId) {
        let src = mem::take(&mut self.src);

        // generate the preamble
        wit_bindgen_core::generated_preamble(&mut self.src, env!("CARGO_PKG_VERSION"));

        // generate the package name
        assert!(self.path.len() > 1);
        let snake = self.path[self.path.len() - 2].to_snake_case();
        self.src.push_str("package ");
        self.src.push_str(&snake);
        self.src.push_str("\n\n");

        // use the Go exposed C API
        let prefix_name = match self.prefix_name.take() {
            Some(name) => name,
            None => "main".to_owned(),
        };
        self.src.push_str(&format!(
            "import _ \"{prefix_name}/{C_GEN_FILES_PATH}\"\n\n"
        ));

        // generate CGo preamble
        self.src.push_str("// #cgo CFLAGS: -I");
        self.src.push_str(&self.c_files_path);
        self.src.push_str("\n");

        self.src.push_str("// #include \"");
        self.src.push_str(self.world.to_snake_case().as_str());
        self.src.push_str(".h\"\n");

        if self.preamble.len() > 0 {
            self.src.append_src(&self.preamble);
        }
        self.src.push_str("import \"C\"\n");
        let world = &resolve.worlds[id];

        self.src.push_str(&src);
    }
}

impl IntoIterator for Packages {
    type Item = (String, Package);
    type IntoIter = std::collections::hash_map::IntoIter<String, Package>;

    fn into_iter(self) -> Self::IntoIter {
        self.packages.into_iter()
    }
}

impl<'a> IntoIterator for &'a Packages {
    type Item = (&'a String, &'a Package);
    type IntoIter = std::collections::hash_map::Iter<'a, String, Package>;

    fn into_iter(self) -> Self::IntoIter {
        self.packages.iter()
    }
}

impl<'a> IntoIterator for &'a mut Packages {
    type Item = (&'a String, &'a mut Package);
    type IntoIter = std::collections::hash_map::IterMut<'a, String, Package>;

    fn into_iter(self) -> Self::IntoIter {
        self.packages.iter_mut()
    }
}
