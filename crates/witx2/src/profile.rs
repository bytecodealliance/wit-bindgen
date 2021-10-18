use crate::{rewrite_error, Docs, Interface};
use anyhow::{bail, Context, Result};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

mod ast;

/// Represents the kind of file being loaded.
#[derive(Debug, Clone, Copy)]
pub enum LoadKind {
    /// The load is for a profile.
    Profile,
    /// The load is for an interface.
    Interface,
}

/// Represents a host profile.
#[derive(Clone)]
pub struct Profile {
    provides: HashMap<String, Provide>,
    requires: HashMap<String, Require>,
    implements: HashMap<String, Implement>,
}

impl Profile {
    /// Gets the interfaces provided by the profile.
    pub fn provides(&self) -> impl Iterator<Item = &Provide> {
        self.provides.values()
    }

    /// Gets the interfaces required by the profile.
    pub fn requires(&self) -> impl Iterator<Item = &Require> {
        self.requires.values()
    }

    /// Gets the interface implementations specified by the profile.
    pub fn implements(&self) -> impl Iterator<Item = &Implement> {
        self.implements.values()
    }

    /// Parse a profile given a file path and contents.
    pub fn parse(path: impl AsRef<Path>, contents: &str) -> Result<Self> {
        let path = path.as_ref();
        Self::parse_with(path, contents, Self::load_from(Self::root(path)?))
    }

    /// Parse a profile given a path to a file.
    pub fn parse_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read profile `{}`", path.display()))?;

        Self::parse_with(path, &contents, Self::load_from(Self::root(path)?))
    }

    /// Parse a profile given a file path, content, and load function.
    pub fn parse_with(
        path: impl AsRef<Path>,
        contents: &str,
        mut load: impl FnMut(LoadKind, &str) -> Result<(PathBuf, String)>,
    ) -> Result<Self> {
        Self::_parse_with(
            path.as_ref(),
            contents,
            &mut load,
            &mut HashSet::new(),
            &mut HashSet::new(),
            &mut HashMap::new(),
        )
    }

    fn load_from(root: PathBuf) -> impl FnMut(LoadKind, &str) -> Result<(PathBuf, String)> {
        move |kind, name| {
            let (ext, desc) = match kind {
                LoadKind::Profile => ("profile", "profile"),
                LoadKind::Interface => ("witx", "interface"),
            };

            let path = root.join(name).with_extension(ext);
            let contents = fs::read_to_string(&path).context(format!(
                "failed to read {} `{}`",
                desc,
                path.display()
            ))?;

            Ok((path, contents))
        }
    }

    fn root(path: &Path) -> Result<PathBuf> {
        match path.parent() {
            Some(p) => Ok(p.into()),
            None => Ok(std::env::current_dir().context("failed to retrieve current directory")?),
        }
    }

    fn _parse_with(
        path: &Path,
        contents: &str,
        load: &mut dyn FnMut(LoadKind, &str) -> Result<(PathBuf, String)>,
        visiting: &mut HashSet<PathBuf>,
        profiles: &mut HashSet<String>,
        interfaces: &mut HashMap<String, Rc<Interface>>,
    ) -> Result<Self> {
        fn add_interface(
            name: &str,
            interfaces: &mut HashMap<String, Rc<Interface>>,
            load: &mut dyn FnMut(LoadKind, &str) -> Result<(PathBuf, String)>,
        ) -> Result<Rc<Interface>> {
            Ok(match interfaces.entry(name.to_string()) {
                Entry::Occupied(e) => e.get().clone(),
                Entry::Vacant(e) => {
                    let (path, contents) = load(LoadKind::Interface, name)?;

                    e.insert(Rc::new(Interface::parse_with(path, &contents, |name| {
                        load(LoadKind::Interface, name)
                    })?))
                    .clone()
                }
            })
        }

        // Parse the `contents `into an AST
        let ast = match ast::Ast::parse(contents) {
            Ok(ast) => ast,
            Err(mut e) => {
                let file = path.display().to_string();
                rewrite_error(&mut e, &file, contents);
                return Err(e);
            }
        };

        if !visiting.insert(path.to_path_buf()) {
            bail!("file `{}` recursively extends itself", path.display());
        }

        let mut provides = HashMap::new();
        let mut requires = HashMap::new();
        let mut implements = HashMap::new();

        for item in ast.items {
            match item {
                ast::Item::Extend(e) => {
                    if profiles.contains(e.profile.as_ref()) {
                        continue;
                    }

                    let (path, contents) = load(LoadKind::Profile, &e.profile)?;

                    let profile =
                        Self::_parse_with(&path, &contents, load, visiting, profiles, interfaces)?;

                    profiles.insert(e.profile.to_string());

                    for (name, interface) in profile.provides {
                        provides.insert(name, interface);
                    }

                    for (name, interface) in profile.requires {
                        requires.insert(name, interface);
                    }
                }
                ast::Item::Provide(p) => {
                    provides.insert(
                        p.interface.to_string(),
                        Provide {
                            docs: p.docs.docs.iter().into(),
                            interface: add_interface(&p.interface, interfaces, load)?,
                        },
                    );
                }
                ast::Item::Require(r) => {
                    requires.insert(
                        r.interface.to_string(),
                        Require {
                            docs: r.docs.docs.iter().into(),
                            interface: add_interface(&r.interface, interfaces, load)?,
                        },
                    );
                }
                ast::Item::Implement(i) => {
                    if let Some(existing) = implements.insert(
                        i.interface.to_string(),
                        Implement {
                            docs: i.docs.docs.iter().into(),
                            interface: i.interface.to_string(),
                            component: (*i.component).to_owned(),
                        },
                    ) {
                        if existing.component != i.component {
                            bail!(
                                "interface `{}` cannot be implemented by both `{}` and `{}`",
                                i.interface,
                                existing.component,
                                i.component
                            );
                        }
                    }
                }
            }
        }

        visiting.remove(path);

        Ok(Self {
            provides,
            requires,
            implements,
        })
    }
}

/// Represents a provided interface specified by a host profile.
#[derive(Clone)]
pub struct Provide {
    pub docs: Docs,
    interface: Rc<Interface>,
}

impl Provide {
    /// Gets the interface provided by the host.
    pub fn interface(&self) -> &Interface {
        self.interface.as_ref()
    }
}

/// Represents a required interface specified by a host profile.
#[derive(Clone)]
pub struct Require {
    pub docs: Docs,
    interface: Rc<Interface>,
}

impl Require {
    /// Gets the interface required by the host.
    pub fn interface(&self) -> &Interface {
        self.interface.as_ref()
    }
}

/// Represents an interface implementation default specified by a host profile.
#[derive(Clone)]
pub struct Implement {
    pub docs: Docs,
    pub interface: String,
    pub component: String,
}

#[cfg(test)]
mod test {}
