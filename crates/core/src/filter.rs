use anyhow::{Result, bail};
use std::{
    collections::HashSet,
    fmt::{self, Display},
};
use wit_parser::{Function, Resolve, WorldKey};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct FilterRule<M: FilterMode, F: FilterTarget> {
    pub mode: M,
    pub filter: F,
}

impl<M: FilterMode, F: FilterTarget> fmt::Display for FilterRule<M, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.mode.fmt_prefix(f)?;
        self.filter.fmt(f)
    }
}

impl<M: FilterMode, F: FilterTarget> FilterRule<M, F> {
    pub fn parse(s: &str) -> Self {
        let (mode, rest) = M::parse(s);

        Self {
            mode,
            filter: F::parse(rest),
        }
    }
}

pub trait FilterMode: Sized {
    fn parse(s: &str) -> (Self, &str);
    fn fmt_prefix(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

pub trait FilterTarget: Display + Sized {
    fn parse(s: &str) -> Self;
    fn all() -> Self;
    fn is_all(&self) -> bool;
}

pub trait FilterSet: Sized {
    type Mode: FilterMode;
    type Filter: FilterTarget;

    fn new(rules: Vec<FilterRule<Self::Mode, Self::Filter>>) -> Self;

    fn rules(&self) -> &[FilterRule<Self::Mode, Self::Filter>];
    fn rules_mut(&mut self) -> &mut Vec<FilterRule<Self::Mode, Self::Filter>>;
    fn used_options(&self) -> &HashSet<usize>;
    fn used_options_mut(&mut self) -> &mut HashSet<usize>;

    fn option_name() -> &'static str;

    fn all(mode: Self::Mode) -> Self {
        Self::new(vec![FilterRule {
            mode,
            filter: Self::Filter::all(),
        }])
    }

    fn push(&mut self, directive: &str) {
        self.rules_mut()
            .push(FilterRule::<Self::Mode, Self::Filter>::parse(directive));
    }

    fn apply_rules(
        &mut self,
        resolve: &Resolve,
        interface: Option<&WorldKey>,
        func: &Function,
        is_import: bool,
    ) -> Self::Mode;

    /// Tests whether all options were used throughout bindings
    /// generation, returning an error if any were unused.
    fn ensure_all_used(&self) -> Result<()> {
        for (i, opt) in self.rules().iter().enumerate() {
            if self.used_options().contains(&i) {
                continue;
            }
            if !opt.filter.is_all() {
                bail!("unused {}: {opt}", Self::option_name());
            }
        }
        Ok(())
    }

    /// Intended to be used in the header comment of generated code to help
    /// indicate what options were specified.
    fn debug_opts(&self) -> impl Iterator<Item = String> + '_ {
        self.rules().iter().map(|opt| opt.to_string())
    }
}
