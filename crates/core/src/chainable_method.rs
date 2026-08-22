use std::fmt;
use std::{collections::HashSet, fmt::Write};
use wit_parser::{Function, FunctionKind, Resolve, WorldKey};

use crate::filter::{FilterMode, FilterRule, FilterSet, FilterTarget};

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
    /// - `foo:bar/baz#my-resource` - enable chaining for all methods in a resource
    /// - `foo:bar/baz#my-resource.some-method` - enable chaining for particular method
    ///
    /// Each filter may also have one of two modifier prefixes:
    /// - `-` - inverts the selection; e.g. `-all` will disable chaining for all
    /// - `&` - makes the chainable return `&Self` instead of `Self` (borrowing)
    ///
    /// For instance, `&foo:bar/baz#my-resource` will make all methods in said resource
    /// borrowing chainable, while `-foo:bar/baz#my-resource.some-method` will disable it
    /// for that particular method.
    ///
    /// Options are processed in the order they are passed here, so if a method
    /// matches two directives passed the least-specific one should be last.
    #[cfg_attr(
        feature = "clap",
        arg(
            long = "chainable-methods",
            value_parser = parse_chainable_method,
            value_delimiter = ',',
            value_name = "FILTER",
        )
    )]
    chainable_methods: Vec<ChainableMethod>,

    #[cfg_attr(feature = "clap", arg(skip))]
    #[cfg_attr(feature = "serde", serde(skip))]
    used_options: HashSet<usize>,
}

impl FilterSet for ChainableMethodFilterSet {
    type Mode = Option<ChainingMode>;
    type Filter = ChainableMethodFilter;

    fn new(rules: Vec<ChainableMethod>) -> Self {
        Self {
            chainable_methods: rules,
            used_options: HashSet::new(),
        }
    }

    fn rules(&self) -> &[ChainableMethod] {
        &self.chainable_methods
    }
    fn rules_mut(&mut self) -> &mut Vec<ChainableMethod> {
        &mut self.chainable_methods
    }
    fn used_options(&self) -> &HashSet<usize> {
        &self.used_options
    }
    fn used_options_mut(&mut self) -> &mut HashSet<usize> {
        &mut self.used_options
    }

    fn option_name() -> &'static str {
        "chainable"
    }

    fn apply_rules(
        &mut self,
        resolve: &Resolve,
        interface: Option<&WorldKey>,
        func: &Function,
        is_import: bool,
    ) -> Option<ChainingMode> {
        if !is_import || func.result.is_some() {
            return None;
        }

        let resource = match func.kind {
            FunctionKind::AsyncMethod(r) | FunctionKind::Method(r) => r,
            _ => return None,
        };

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
            let matched = match &opt.filter {
                ChainableMethodFilter::All => true,
                ChainableMethodFilter::Resource(s) => *s == resource_name_to_test,
                ChainableMethodFilter::Method(s) => *s == method_name_to_test,
            };

            if matched {
                self.used_options.insert(i);
                return opt.mode;
            }
        }

        None
    }
}

#[cfg(feature = "clap")]
fn parse_chainable_method(s: &str) -> Result<ChainableMethod, String> {
    Ok(ChainableMethod::parse(s))
}

#[derive(Clone, Copy, Debug)]
pub enum ChainingMode {
    Owning,
    Borrowing,
}

type ChainableMethod = FilterRule<Option<ChainingMode>, ChainableMethodFilter>;

impl FilterMode for Option<ChainingMode> {
    fn parse(s: &str) -> (Self, &str) {
        match s.strip_prefix('-') {
            Some(rest) => (None, rest),
            None => match s.strip_prefix('&') {
                Some(rest) => (Some(ChainingMode::Borrowing), rest),
                None => (Some(ChainingMode::Owning), s),
            },
        }
    }
    fn fmt_prefix(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Some(ChainingMode::Owning) => {}
            Some(ChainingMode::Borrowing) => f.write_char('&')?,
            None => f.write_char('-')?,
        };
        Ok(())
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum ChainableMethodFilter {
    All,
    Resource(String),
    Method(String),
}

impl FilterTarget for ChainableMethodFilter {
    fn parse(s: &str) -> Self {
        match s {
            "all" => ChainableMethodFilter::All,
            other => {
                if other.contains("[method]") {
                    ChainableMethodFilter::Method(other.to_string())
                } else {
                    ChainableMethodFilter::Resource(other.to_string())
                }
            }
        }
    }

    fn all() -> ChainableMethodFilter {
        ChainableMethodFilter::All
    }

    fn is_all(&self) -> bool {
        matches!(self, ChainableMethodFilter::All)
    }
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
