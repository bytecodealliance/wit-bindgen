use anyhow::{Result, bail};
use std::collections::HashSet;
use std::fmt;
use wit_parser::{Function, FunctionKind, Resolve, WorldKey};

/// Structure used to parse the command line argument `--sync` consistently
/// across guest generators.
#[cfg_attr(feature = "clap", derive(clap::Parser))]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[derive(Clone, Default, Debug)]
pub struct AsyncFilterSet {
    /// Determines which `async` functions to lift or lower synchronously, if
    /// any.
    ///
    /// This option can be passed multiple times and additionally accepts
    /// comma-separated values for each option passed. Each individual argument
    /// passed here can be one of:
    ///
    /// - `all` - all imports and exports will be sync
    /// - `foo:bar/baz#method` - force this method to be sync
    /// - `import:foo:bar/baz#method` - force this method to be sync, but only
    ///   as an import
    /// - `export:foo:bar/baz#method` - force this method to be sync, but only
    ///   as an export
    ///
    /// Functions defined as `async` in WIT get async bindings unless they are
    /// listed here. Functions not defined as `async` always get sync bindings,
    /// as the component model does not allow lifting or lowering them async.
    #[cfg_attr(
        feature = "clap",
        arg(
            long = "sync",
            value_parser = parse_sync,
            value_delimiter =',',
            value_name = "FILTER",
        ),
    )]
    #[cfg_attr(feature = "serde", serde(rename = "sync"))]
    sync: Vec<SyncFilter>,

    #[cfg_attr(feature = "clap", arg(skip))]
    #[cfg_attr(feature = "serde", serde(skip))]
    used_options: HashSet<usize>,
}

#[cfg(feature = "clap")]
fn parse_sync(s: &str) -> Result<SyncFilter, String> {
    Ok(SyncFilter::parse(s))
}

impl AsyncFilterSet {
    /// Returns whether the `func` provided is to be bound `async` or not.
    pub fn is_async(
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
        let mut sync = false;
        for (i, filter) in self.sync.iter().enumerate() {
            let matches = match filter {
                SyncFilter::All => true,
                SyncFilter::Function(s) => *s == name_to_test,
                SyncFilter::Import(s) => is_import && *s == name_to_test,
                SyncFilter::Export(s) => !is_import && *s == name_to_test,
            };
            if matches {
                self.used_options.insert(i);
                sync = true;
            }
        }
        if sync {
            return false;
        }

        match &func.kind {
            FunctionKind::Freestanding
            | FunctionKind::Method(_)
            | FunctionKind::Static(_)
            | FunctionKind::Constructor(_)
            | FunctionKind::Getter
            | FunctionKind::MethodGetter(_)
            | FunctionKind::StaticGetter(_)
            | FunctionKind::Setter
            | FunctionKind::MethodSetter(_)
            | FunctionKind::StaticSetter(_) => false,
            FunctionKind::AsyncFreestanding
            | FunctionKind::AsyncMethod(_)
            | FunctionKind::AsyncStatic(_) => true,
        }
    }

    /// Intended to be used in the header comment of generated code to help
    /// indicate what options were specified.
    pub fn debug_opts(&self) -> impl Iterator<Item = String> + '_ {
        self.sync.iter().map(|filter| filter.to_string())
    }

    /// Tests whether all `--sync` options were used throughout bindings
    /// generation, returning an error if any were unused.
    pub fn ensure_all_used(&self) -> Result<()> {
        for (i, filter) in self.sync.iter().enumerate() {
            if self.used_options.contains(&i) {
                continue;
            }
            if !matches!(filter, SyncFilter::All) {
                bail!("unused sync option: {filter}");
            }
        }
        Ok(())
    }

    /// Pushes a new option into this set.
    pub fn push(&mut self, directive: &str) {
        self.sync.push(SyncFilter::parse(directive));
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
enum SyncFilter {
    All,
    Function(String),
    Import(String),
    Export(String),
}

impl SyncFilter {
    fn parse(s: &str) -> SyncFilter {
        match s {
            "all" => SyncFilter::All,
            other => match other.strip_prefix("import:") {
                Some(s) => SyncFilter::Import(s.to_string()),
                None => match other.strip_prefix("export:") {
                    Some(s) => SyncFilter::Export(s.to_string()),
                    None => SyncFilter::Function(s.to_string()),
                },
            },
        }
    }
}

impl fmt::Display for SyncFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncFilter::All => write!(f, "all"),
            SyncFilter::Function(s) => write!(f, "{s}"),
            SyncFilter::Import(s) => write!(f, "import:{s}"),
            SyncFilter::Export(s) => write!(f, "export:{s}"),
        }
    }
}
