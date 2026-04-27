use anyhow::{Result, bail};
use std::collections::HashSet;
use std::fmt;
use wit_parser::{Function, FunctionKind, Resolve, WorldKey};

/// Structure used to parse the command line argument `--chainable-method` consistently
/// across guest generators.
#[cfg_attr(feature = "clap", derive(clap::Parser))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[derive(Clone, Default, Debug)]
pub struct ChainableMethodFilterSet {
    /// Determines which resource methods should have chaining enabled.
    /// Chaining takes a WIT method import returning nothing, and modifies bindgen
    /// in a language-dependent way to return `self` in the glue code. This does
    /// not affect the ABI in any way.
    ///
    /// This option can be passed multiple times and additionally accepts
    /// comma-separated values for each option passed. Each individual argument
    /// passed here can be one of:
    ///
    /// - `all` - all applicable methods will be chainable
    /// - `-all` - no methods will be chainable
    /// - `foo:bar/baz#my-resource` - enable chaining for all methods in a resource
    /// - `foo:bar/baz#my-resource.some-method` - enable chaining for particular method
    ///
    /// Options are processed in the order they are passed here, so if a method
    /// matches two directives passed the least-specific one should be last.
    #[cfg_attr(
        feature = "clap",
        arg(
            long = "chainable-methods",
            value_parser = parse_chainable_method,
            value_delimiter =',',
            value_name = "FILTER",
        ),
    )]
    chainable_methods: Vec<ChainableMethod>,

    #[cfg_attr(feature = "clap", arg(skip))]
    #[cfg_attr(feature = "serde", serde(skip))]
    used_options: HashSet<usize>,
}

#[cfg(feature = "clap")]
fn parse_chainable_method(s: &str) -> Result<ChainableMethod, String> {
    Ok(ChainableMethod::parse(s))
}

impl ChainableMethodFilterSet {
    /// Returns a set where all functions should be chainable or not depending on
    /// `enable` provided.
    pub fn all(enable: bool) -> ChainableMethodFilterSet {
        ChainableMethodFilterSet {
            chainable_methods: vec![ChainableMethod {
                enabled: enable,
                filter: ChainableMethodFilter::All,
            }],
            used_options: HashSet::new(),
        }
    }

    /// Returns whether the `func` provided should be made chainable
    pub fn should_be_chainable(
        &mut self,
        resolve: &Resolve,
        interface: Option<&WorldKey>,
        func: &Function,
        is_import: bool,
    ) -> bool {
        if !is_import {
            return false;
        }

        if func.result.is_some() {
            return false;
        }

        match func.kind {
            FunctionKind::AsyncMethod(resource) | FunctionKind::Method(resource) => {
                let interface_name = match interface.map(|key| resolve.name_world_key(key)) {
                    Some(str) => str + "#",
                    None => "".into(),
                };

                let resource_name_to_test = format!(
                    "{}{}",
                    interface_name,
                    resolve.types[resource].name.as_ref().unwrap()
                );

                let method_name_to_test = format!("{}{}", interface_name, func.name);

                for (i, opt) in self.chainable_methods.iter().enumerate() {
                    match &opt.filter {
                        ChainableMethodFilter::All => {
                            self.used_options.insert(i);
                            return opt.enabled;
                        }
                        ChainableMethodFilter::Resource(s) => {
                            if *s == resource_name_to_test {
                                self.used_options.insert(i);
                                return opt.enabled;
                            }
                        }
                        ChainableMethodFilter::Method(s) => {
                            if *s == method_name_to_test {
                                self.used_options.insert(i);
                                return opt.enabled;
                            }
                        }
                    };
                }

                return false;
            }
            _ => {
                return false;
            }
        }
    }

    /// Intended to be used in the header comment of generated code to help
    /// indicate what options were specified.
    pub fn debug_opts(&self) -> impl Iterator<Item = String> + '_ {
        self.chainable_methods.iter().map(|opt| opt.to_string())
    }

    /// Tests whether all `--chainable-method` options were used throughout bindings
    /// generation, returning an error if any were unused.
    pub fn ensure_all_used(&self) -> Result<()> {
        for (i, opt) in self.chainable_methods.iter().enumerate() {
            if self.used_options.contains(&i) {
                continue;
            }
            if !matches!(opt.filter, ChainableMethodFilter::All) {
                bail!("unused chainable option: {opt}");
            }
        }
        Ok(())
    }

    /// Pushes a new option into this set.
    pub fn push(&mut self, directive: &str) {
        self.chainable_methods
            .push(ChainableMethod::parse(directive));
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
struct ChainableMethod {
    enabled: bool,
    filter: ChainableMethodFilter,
}

impl ChainableMethod {
    fn parse(s: &str) -> ChainableMethod {
        let (s, enabled) = match s.strip_prefix('-') {
            Some(s) => (s, false),
            None => (s, true),
        };
        let filter = match s {
            "all" => ChainableMethodFilter::All,
            other => {
                if other.contains("[method]") {
                    ChainableMethodFilter::Method(other.to_string())
                } else {
                    ChainableMethodFilter::Resource(other.to_string())
                }
            }
        };
        ChainableMethod { enabled, filter }
    }
}

impl fmt::Display for ChainableMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.enabled {
            write!(f, "-")?;
        }
        self.filter.fmt(f)
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
enum ChainableMethodFilter {
    All,
    Resource(String),
    Method(String),
}

impl fmt::Display for ChainableMethodFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainableMethodFilter::All => write!(f, "all"),
            ChainableMethodFilter::Resource(s) => write!(f, "{s}"),
            ChainableMethodFilter::Method(s) => write!(f, "{s}"),
        }
    }
}
