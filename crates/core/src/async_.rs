use std::fmt;
use std::fmt::Write;
use wit_parser::{Function, FunctionKind, Resolve, WorldKey};

use crate::define_filter_set;
use crate::filter::{FilterMode, FilterRule, FilterTarget};

define_filter_set! {
    /// Structure used to parse the command line argument `--async` consistently
    /// across guest generators.
    pub struct AsyncFilterSet,
    /// Determines which functions to lift or lower `async`, if any.
    ///
    /// This option can be passed multiple times and additionally accepts
    /// comma-separated values for each option passed. Each individual argument
    /// passed here can be one of:
    ///
    /// - `all` - all imports and exports will be async
    ///
    /// - `-all` - force all imports and exports to be sync
    ///
    /// - `foo:bar/baz#method` - force this method to be async
    ///
    /// - `import:foo:bar/baz#method` - force this method to be async, but only
    ///   as an import
    ///
    /// - `-export:foo:bar/baz#method` - force this export to be sync
    ///
    ///
    /// If a method is not listed in this option then the WIT's default bindings
    /// mode will be used. If the WIT function is defined as `async` then async
    /// bindings will be generated, otherwise sync bindings will be generated.
    ///
    /// Options are processed in the order they are passed here, so if a method
    /// matches two directives passed the least-specific one should be last.
    bool, AsyncFilter,
    "async"
}

impl AsyncFilterSet {
    pub fn any_enabled(&self) -> bool {
        self.rules.iter().any(|o| o.mode)
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

        for (i, opt) in self.rules.iter().enumerate() {
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

impl FilterMode for bool {
    fn parse(s: &str) -> (Self, &str) {
        match s.strip_prefix('-') {
            Some(rest) => (false, rest),
            None => (true, s),
        }
    }
    fn fmt_prefix(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !*self {
            f.write_char('-')?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
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
