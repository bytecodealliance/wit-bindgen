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
#[derive(Default, Debug, Clone)]
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
        let mut parsed = Self::default();

        let mut visiting = HashSet::new();
        visiting.insert(path.as_ref().into());

        Self::_parse(
            path.as_ref(),
            contents,
            &mut load,
            &mut visiting,
            &mut HashSet::new(),
            &mut HashMap::new(),
            &mut parsed,
        )
        .map_err(|mut e| {
            let file = path.as_ref().display().to_string();
            rewrite_error(&mut e, &file, contents);
            e
        })?;

        Ok(parsed)
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

    fn _parse(
        path: &Path,
        contents: &str,
        load: &mut dyn FnMut(LoadKind, &str) -> Result<(PathBuf, String)>,
        visiting: &mut HashSet<PathBuf>,
        profiles: &mut HashSet<String>,
        interfaces: &mut HashMap<String, Rc<Interface>>,
        parsed: &mut Self,
    ) -> Result<()> {
        fn load_interface(
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

        let ast = ast::Ast::parse(contents)?;
        let mut extending = true;

        for item in ast.items {
            match item {
                ast::Item::Extend(e) => {
                    if !extending {
                        bail!(crate::Error {
                            span: e.span,
                            msg: "extend statements must come before all other statements"
                                .to_string(),
                        });
                    }

                    if !profiles.insert(e.profile.to_string()) {
                        continue;
                    }

                    let (path, contents) = load(LoadKind::Profile, &e.profile)?;

                    if !visiting.insert(path.clone()) {
                        bail!(crate::Error {
                            span: e.span,
                            msg: format!(
                                "extending `{}` ({}) forms a cycle",
                                e.profile,
                                path.display()
                            )
                        });
                    }

                    Self::_parse(
                        &path, &contents, load, visiting, profiles, interfaces, parsed,
                    )?;
                }
                ast::Item::Provide(p) => {
                    extending = false;
                    parsed.provides.insert(
                        p.interface.to_string(),
                        Provide {
                            docs: p.docs.docs.iter().into(),
                            interface: load_interface(&p.interface, interfaces, load)?,
                        },
                    );
                }
                ast::Item::Require(r) => {
                    extending = false;
                    parsed.requires.insert(
                        r.interface.to_string(),
                        Require {
                            docs: r.docs.docs.iter().into(),
                            interface: load_interface(&r.interface, interfaces, load)?,
                        },
                    );
                }
                ast::Item::Implement(i) => {
                    extending = false;
                    if let Some(existing) = parsed.implements.insert(
                        i.interface.to_string(),
                        Implement {
                            docs: i.docs.docs.iter().into(),
                            interface: i.interface.to_string(),
                            component: (*i.component).to_owned(),
                        },
                    ) {
                        if existing.component != i.component {
                            bail!(crate::Error {
                                span: i.span,
                                msg: format!(
                                    "interface `{}` is already implemented by `{}`",
                                    i.interface, existing.component
                                ),
                            });
                        }
                    }
                }
            }
        }

        visiting.remove(path);

        Ok(())
    }
}

/// Represents a provided interface specified by a host profile.
#[derive(Debug, Clone)]
pub struct Provide {
    pub docs: Docs,
    pub interface: Rc<Interface>,
}

impl Provide {
    /// Gets the interface provided by the host.
    pub fn interface(&self) -> &Interface {
        self.interface.as_ref()
    }
}

/// Represents a required interface specified by a host profile.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct Implement {
    pub docs: Docs,
    pub interface: String,
    pub component: String,
}

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn it_parses_empty_content() -> Result<()> {
        let profile = Profile::parse("test.profile", "")?;
        assert!(profile.provides.is_empty());
        assert!(profile.requires.is_empty());
        assert!(profile.implements.is_empty());
        Ok(())
    }

    #[test]
    fn it_parses_whitespace() -> Result<()> {
        let profile = Profile::parse("test.profile", "       ")?;
        assert!(profile.provides.is_empty());
        assert!(profile.requires.is_empty());
        assert!(profile.implements.is_empty());
        Ok(())
    }

    #[test]
    fn it_parses_comments() -> Result<()> {
        let profile = Profile::parse("test.profile", "// a comment\n")?;
        assert!(profile.provides.is_empty());
        assert!(profile.requires.is_empty());
        assert!(profile.implements.is_empty());
        Ok(())
    }

    #[test]
    fn it_fails_on_invalid_syntax() -> Result<()> {
        let e = Profile::parse("test.profile", "invalid").expect_err("expected parsing to fail");
        assert_eq!(e.to_string(), "unexpected character 'i'\n     --> test.profile:1:1\n      |\n    1 | invalid\n      | ^");
        Ok(())
    }

    #[test]
    fn it_parses_a_profile() -> Result<()> {
        let tmpdir = tempdir()?;

        let base_contents = r#"
            // foo from base
            require "foo"
            // base
            provide "base"
        "#;

        let contents = r#"
            extend "base"
            // foo
            require "foo"
            // quz
            provide "quz"
            require "bar"
            require "baz"
            provide "qux"
            // i with c
            implement "i" with "c"
            implement "i2" with "c2"
        "#;

        fn verify(profile: Profile) {
            assert_eq!(profile.provides.len(), 3);
            let p = &profile.provides["quz"];
            assert_eq!(p.docs.contents.as_deref(), Some("quz"));
            assert_eq!(p.interface.name, "quz");
            let p = &profile.provides["qux"];
            assert!(p.docs.contents.is_none());
            assert_eq!(p.interface.name, "qux");
            let p = &profile.provides["base"];
            assert_eq!(p.docs.contents.as_deref(), Some("base"));
            assert_eq!(p.interface.name, "base");

            assert_eq!(profile.requires.len(), 3);
            let r = &profile.requires["foo"];
            assert_eq!(r.docs.contents.as_deref(), Some("foo"));
            assert_eq!(r.interface.name, "foo");
            let r = &profile.requires["bar"];
            assert!(r.docs.contents.is_none());
            assert_eq!(r.interface.name, "bar");
            let r = &profile.requires["baz"];
            assert!(r.docs.contents.is_none());
            assert_eq!(r.interface.name, "baz");

            assert_eq!(profile.implements.len(), 2);
            let i = &profile.implements["i"];
            assert_eq!(i.docs.contents.as_deref(), Some("i with c"));
            assert_eq!(i.interface, "i");
            assert_eq!(i.component, "c");
            let i = &profile.implements["i2"];
            assert!(i.docs.contents.is_none());
            assert_eq!(i.interface, "i2");
            assert_eq!(i.component, "c2");
        }

        let path = tmpdir.path().join("test.profile");

        fs::write(tmpdir.path().join(&path), contents)?;
        fs::write(tmpdir.path().join("foo.witx"), "")?;
        fs::write(tmpdir.path().join("quz.witx"), "")?;
        fs::write(tmpdir.path().join("bar.witx"), "")?;
        fs::write(tmpdir.path().join("baz.witx"), "")?;
        fs::write(tmpdir.path().join("qux.witx"), "")?;
        fs::write(tmpdir.path().join("base.witx"), "")?;
        fs::write(tmpdir.path().join("base.profile"), base_contents)?;

        // Test from a file
        verify(Profile::parse_file(&path)?);

        // Test from a string
        verify(Profile::parse(&path, contents)?);

        // Test with a custom load function
        verify(Profile::parse_with(path, contents, |kind, name| {
            Ok(match (kind, name) {
                (LoadKind::Interface, "foo") => ("foo.witx".into(), "".to_string()),
                (LoadKind::Interface, "quz") => ("quz.witx".into(), "".to_string()),
                (LoadKind::Interface, "bar") => ("bar.witx".into(), "".to_string()),
                (LoadKind::Interface, "baz") => ("baz.witx".into(), "".to_string()),
                (LoadKind::Interface, "qux") => ("qux.witx".into(), "".to_string()),
                (LoadKind::Interface, "base") => ("base.witx".into(), "".to_string()),
                (LoadKind::Interface, _) => panic!("unexpected interface load of `{}`", name),
                (LoadKind::Profile, "base") => ("base.profile".into(), base_contents.to_string()),
                (LoadKind::Profile, _) => panic!("unexpected profile load of `{}`", name),
            })
        })?);

        Ok(())
    }

    #[test]
    fn it_fails_with_out_of_order_extend() -> Result<()> {
        let e = Profile::parse_with(
            "test.profile",
            r#"
            implement "foo" with "bar"
            extend "foo"
        "#,
            |kind, name| {
                Ok(match (kind, name) {
                    (LoadKind::Profile, "foo") => ("foo.profile".into(), "".to_string()),
                    _ => panic!("unexpected load"),
                })
            },
        )
        .expect_err("expected parsing to fail");
        assert_eq!(e.to_string(), "extend statements must come before all other statements\n     --> test.profile:3:13\n      |\n    3 |             extend \"foo\"\n      |             ^-----------");
        Ok(())
    }

    #[test]
    fn it_fails_with_extend_cycle() -> Result<()> {
        let e = Profile::parse_with(
            "test.profile",
            r#"
            extend "test"
        "#,
            |kind, name| {
                Ok(match (kind, name) {
                    (LoadKind::Profile, "test") => ("test.profile".into(), "".to_string()),
                    _ => panic!("unexpected load"),
                })
            },
        )
        .expect_err("expected parsing to fail");
        assert_eq!(e.to_string(), "extending `test` (test.profile) forms a cycle\n     --> test.profile:2:13\n      |\n    2 |             extend \"test\"\n      |             ^------------");
        Ok(())
    }

    #[test]
    fn it_fails_with_conflicting_implementations() -> Result<()> {
        let e = Profile::parse(
            "test.profile",
            r#"
            implement "foo" with "bar"
            implement "foo" with "baz"
        "#,
        )
        .expect_err("expected parsing to fail");
        assert_eq!(e.to_string(), "interface `foo` is already implemented by `bar`\n     --> test.profile:3:13\n      |\n    3 |             implement \"foo\" with \"baz\"\n      |             ^-------------------------");
        Ok(())
    }
}
