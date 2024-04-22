use heck::ToSnakeCase;
use wit_bindgen_core::{
    name_package_module,
    wit_parser::{Resolve, WorldKey},
    Direction,
};

pub(crate) trait GoPath {
    fn to_path(&self, resolve: &Resolve, direction: Direction) -> Vec<String>;
}

impl GoPath for WorldKey {
    fn to_path(&self, resolve: &Resolve, direction: Direction) -> Vec<String> {
        let mut path = Vec::new();
        if matches!(direction, Direction::Export) {
            path.push("exports".to_string());
        }
        match self {
            WorldKey::Name(n) => path.push(n.to_snake_case()),
            WorldKey::Interface(id) => {
                let iface = &resolve.interfaces[*id];
                let pkg = iface.package.unwrap();
                let pkgname = resolve.packages[pkg].name.clone();
                path.push(pkgname.namespace.to_snake_case());
                path.push(name_package_module(resolve, pkg));
                path.push(iface.name.as_ref().unwrap().to_snake_case());
            }
        }
        path
    }
}
