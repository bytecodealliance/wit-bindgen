use std::fmt;
use std::{collections::HashSet, fmt::Write};
use wit_parser::{Function, FunctionKind, Resolve, WorldKey};

use crate::filter::{FilterMode, FilterRule, FilterSet, FilterTarget};

/// Structure used to parse the command line argument `--async` consistently
/// across guest generators.
#[cfg_attr(feature = "clap", derive(clap::Parser))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[derive(Clone, Default, Debug)]
pub struct AsyncFilterSet {
    /// Determines which functions to lift or lower `async`, if any.
    ///
    /// This option can be passed multiple times and additionally accepts
    /// comma-separated values for each option passed. Each individual argument
    /// passed here can be one of:
    ///
    /// - `all` - all imports and exports will be async
    /// - `-all` - force all imports and exports to be sync
    /// - `foo:bar/baz#method` - force this method to be async
    /// - `import:foo:bar/baz#method` - force this method to be async, but only
    ///   as an import
    /// - `-export:foo:bar/baz#method` - force this export to be sync
    ///
    /// If a method is not listed in this option then the WIT's default bindings
    /// mode will be used. If the WIT function is defined as `async` then async
    /// bindings will be generated, otherwise sync bindings will be generated.
    ///
    /// Options are processed in the order they are passed here, so if a method
    /// matches two directives passed the least-specific one should be last.
    #[cfg_attr(
        feature = "clap",
        arg(
            long = "async",
            value_parser = parse_async,
            value_delimiter = ',',
            value_name = "FILTER",
        )
    )]
    #[cfg_attr(feature = "serde", serde(rename = "async"))]
    async_: Vec<Async>,

    #[cfg_attr(feature = "clap", arg(skip))]
    #[cfg_attr(feature = "serde", serde(skip))]
    used_options: HashSet<usize>,
}

impl FilterSet for AsyncFilterSet {
    type Mode = bool;
    type Filter = AsyncFilter;

    fn new(rules: Vec<Async>) -> Self {
        Self {
            async_: rules,
            used_options: HashSet::new(),
        }
    }

    fn rules(&self) -> &[Async] {
        &self.async_
    }
    fn rules_mut(&mut self) -> &mut Vec<Async> {
        &mut self.async_
    }
    fn used_options(&self) -> &HashSet<usize> {
        &self.used_options
    }
    fn used_options_mut(&mut self) -> &mut HashSet<usize> {
        &mut self.used_options
    }

    fn option_name() -> &'static str {
        "async"
    }

    fn apply_rules(
        &mut self,
        resolve: &Resolve,
        interface: Option<&WorldKey>,
        func: &Function,
        is_import: bool,
    ) -> bool {
        let name_to_test = match interface {
            Some(key) => format!("{}#{}", resolve.name_world_key(key), func.name),
            None => func.name.clone(),
        };

        for (i, opt) in self.async_.iter().enumerate() {
            let name = match &opt.filter {
                AsyncFilter::All => {
                    self.used_options.insert(i);
                    return opt.mode;
                }
                AsyncFilter::Function(s) => s,
                AsyncFilter::Import(s) => {
                    if !is_import {
                        continue;
                    }
                    s
                }
                AsyncFilter::Export(s) => {
                    if is_import {
                        continue;
                    }
                    s
                }
            };
            if *name == name_to_test {
                self.used_options.insert(i);
                return opt.mode;
            }
        }

        matches!(
            func.kind,
            FunctionKind::AsyncFreestanding
                | FunctionKind::AsyncMethod(_)
                | FunctionKind::AsyncStatic(_)
        )
    }
}

#[cfg(feature = "clap")]
fn parse_async(s: &str) -> Result<Async, String> {
    Ok(Async::parse(s))
}

impl AsyncFilterSet {
    pub fn any_enabled(&self) -> bool {
        self.async_.iter().any(|o| o.mode)
    }
}

type Async = FilterRule<bool, AsyncFilter>;

impl FilterMode for bool {
    fn parse(s: &str) -> (Self, &str) {
        match s.strip_prefix('-') {
            Some(rest) => (false, rest),
            None => (true, s),
        }
    }
    fn fmt_prefix(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self {
            f.write_char('-')?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum AsyncFilter {
    All,
    Function(String),
    Import(String),
    Export(String),
}

impl FilterTarget for AsyncFilter {
    fn parse(s: &str) -> Self {
        match s {
            "all" => AsyncFilter::All,
            other => match other.strip_prefix("import:") {
                Some(sub) => AsyncFilter::Import(sub.to_string()),
                None => match other.strip_prefix("export:") {
                    Some(sub) => AsyncFilter::Export(sub.to_string()),
                    None => AsyncFilter::Function(other.to_string()),
                },
            },
        }
    }

    fn all() -> AsyncFilter {
        AsyncFilter::All
    }

    fn is_all(&self) -> bool {
        matches!(self, AsyncFilter::All)
    }
}

impl fmt::Display for AsyncFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AsyncFilter::All => write!(f, "all"),
            AsyncFilter::Function(s) => write!(f, "{s}"),
            AsyncFilter::Import(s) => write!(f, "import:{s}"),
            AsyncFilter::Export(s) => write!(f, "export:{s}"),
        }
    }
}
