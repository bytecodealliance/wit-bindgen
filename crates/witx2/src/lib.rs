use std::fmt;

pub mod abi;
mod ast;
mod interface;
mod lex;
mod profile;
mod sizealign;

pub use interface::*;
pub use profile::*;
pub use sizealign::*;

#[derive(Debug)]
pub struct Error {
    span: lex::Span,
    msg: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.msg.fmt(f)
    }
}

impl std::error::Error for Error {}

fn rewrite_error(err: &mut anyhow::Error, file: &str, contents: &str) {
    #[cfg(feature = "old-witx-compat")]
    if let Some(err) = err.downcast_mut::<wast::Error>() {
        err.set_path(file.as_ref());
        err.set_text(contents);
        return;
    }
    let parse = match err.downcast_mut::<Error>() {
        Some(err) => err,
        None => return lex::rewrite_error(err, file, contents),
    };
    let msg = crate::lex::highlight_err(
        parse.span.start as usize,
        Some(parse.span.end as usize),
        file,
        contents,
        &parse.msg,
    );
    *err = anyhow::anyhow!("{}", msg);
}
