use anyhow::Result;
use std::{
    fmt::{self, Display},
    str::FromStr,
};
use wit_parser::{Function, Resolve, WorldKey};

#[macro_export]
macro_rules! define_filter_set {
    (
        $(#[$struct_meta:meta])*
        pub struct $struct_name:ident,
        $(#[$field_meta:meta])*
        $mode_type:ty,
        $filter_type:ty,
        $option_name:expr
    ) => {
        #[derive(Clone, Default, Debug)]
        #[cfg_attr(feature = "clap", derive(clap::Parser))]
        #[cfg_attr(feature = "serde", derive(serde::Deserialize))]
        $(#[$struct_meta])*
        pub struct $struct_name {
            $(#[$field_meta])*
            #[cfg_attr(
                feature = "clap",
                arg(
                    id = $option_name,
                    long = $option_name,
                    value_delimiter = ',',
                    value_name = "FILTER",
                )
            )]
            #[cfg_attr(feature = "serde", serde(rename = $option_name))]
            rules: Vec<FilterRule<$mode_type, $filter_type>>,

            #[cfg_attr(feature = "clap", arg(skip))]
            #[cfg_attr(feature = "serde", serde(skip))]
            used_options: std::collections::HashSet<usize>,
        }

        impl $crate::filter::FilterSet for $struct_name {
            type Mode = $mode_type;
            type Filter = $filter_type;


            fn all(mode: Self::Mode) -> Self {
                Self {
                    rules: vec![FilterRule {
                        mode,
                        filter: Self::Filter::all(),
                    }],
                    used_options: std::collections::HashSet::new()
                }
            }

            fn push(&mut self, filter: &str) {
                self.rules
                    .push(<FilterRule::<Self::Mode, Self::Filter> as std::str::FromStr>::from_str(filter).unwrap());
            }

            fn option_name() -> &'static str {
                $option_name
            }

            fn apply_rules(
                &mut self,
                resolve: &wit_parser::Resolve,
                interface: Option<&wit_parser::WorldKey>,
                func: &wit_parser::Function,
                is_import: bool,
            ) -> Self::Mode {
                Self::apply_rules(self, resolve, interface, func, is_import)
            }

            fn ensure_all_used(&self) -> anyhow::Result<()> {
                for (i, opt) in self.rules.iter().enumerate() {
                    if self.used_options.contains(&i) {
                        continue;
                    }
                    if opt.filter != Self::Filter::all() {
                        anyhow::bail!("unused {} option: {opt}", Self::option_name());
                    }
                }
                Ok(())
            }

            fn debug_opts(&self) -> impl Iterator<Item = String> + '_ {
                self.rules.iter().map(|opt| opt.to_string())
            }
        }
    };
}

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

impl<M: FilterMode, F: FilterTarget> FromStr for FilterRule<M, F> {
    type Err = String;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        let (mode, rest) = M::parse(s);

        Ok(Self {
            mode,
            filter: F::parse(rest),
        })
    }
}

pub trait FilterMode: Sized {
    fn parse(s: &str) -> (Self, &str);
    fn fmt_prefix(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

pub trait FilterTarget: Display + Sized + PartialEq {
    fn parse(s: &str) -> Self;
    fn all() -> Self;
}

pub trait FilterSet: Sized {
    type Mode: FilterMode;
    type Filter: FilterTarget;

    fn option_name() -> &'static str;

    fn all(mode: Self::Mode) -> Self;

    fn push(&mut self, directive: &str);

    fn apply_rules(
        &mut self,
        resolve: &Resolve,
        interface: Option<&WorldKey>,
        func: &Function,
        is_import: bool,
    ) -> Self::Mode;

    /// Tests whether all options were used throughout bindings
    /// generation, returning an error if any were unused.
    fn ensure_all_used(&self) -> Result<()>;

    /// Intended to be used in the header comment of generated code to help
    /// indicate what options were specified.
    fn debug_opts(&self) -> impl Iterator<Item = String> + '_;
}
