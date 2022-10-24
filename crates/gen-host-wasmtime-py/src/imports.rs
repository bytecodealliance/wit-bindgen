use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use wit_bindgen_core::uwriteln;

/// Tracks all of the import and intrinsics that a given codegen
/// requires and how to generate them when needed.
#[derive(Default)]
pub struct PyImports {
    pyimports: BTreeMap<String, Option<BTreeSet<String>>>,
}

impl PyImports {
    /// Record that a Python import is required
    pub fn pyimport<'a>(&mut self, module: &str, name: impl Into<Option<&'a str>>) {
        let name = name.into();
        let list = self
            .pyimports
            .entry(module.to_string())
            .or_insert(match name {
                Some(_) => Some(BTreeSet::new()),
                None => None,
            });
        match name {
            Some(name) => {
                assert!(list.is_some());
                list.as_mut().unwrap().insert(name.to_string());
            }
            None => assert!(list.is_none()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.pyimports.is_empty()
    }

    pub fn finish(&self) -> String {
        let mut result = String::new();
        for (k, list) in self.pyimports.iter() {
            match list {
                Some(list) => {
                    let list = list.iter().cloned().collect::<Vec<_>>().join(", ");
                    uwriteln!(result, "from {k} import {list}");
                }
                None => uwriteln!(result, "import {k}"),
            }
        }

        if !self.pyimports.is_empty() {
            result.push_str("\n");
        }

        result
    }
}

#[cfg(test)]
mod test {
    use std::collections::{BTreeMap, BTreeSet};

    use super::PyImports;

    #[test]
    fn test_pyimport_only_contents() {
        let mut deps = PyImports::default();
        deps.pyimport("typing", None);
        deps.pyimport("typing", None);
        assert_eq!(deps.pyimports, BTreeMap::from([("typing".into(), None)]));
    }

    #[test]
    fn test_pyimport_only_module() {
        let mut deps = PyImports::default();
        deps.pyimport("typing", "Union");
        deps.pyimport("typing", "List");
        deps.pyimport("typing", "NamedTuple");
        assert_eq!(
            deps.pyimports,
            BTreeMap::from([(
                "typing".into(),
                Some(BTreeSet::from([
                    "Union".into(),
                    "List".into(),
                    "NamedTuple".into()
                ]))
            )])
        );
    }

    #[test]
    #[should_panic]
    fn test_pyimport_conflicting() {
        let mut deps = PyImports::default();
        deps.pyimport("typing", "NamedTuple");
        deps.pyimport("typing", None);
    }
}
