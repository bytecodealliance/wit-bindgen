use crate::imports::PyImports;
use std::fmt::{self, Write};
use std::mem;
use wit_bindgen_core::wit_parser::*;

/// A [Source] represents some unit of Python code
/// and keeps track of its indent.
#[derive(Default)]
pub struct Source {
    body: Body,
    imports: PyImports,
}

#[derive(Default)]
pub struct Body {
    contents: String,
    indent: usize,
}

impl Source {
    /// Appends a string slice to this [Source].
    ///
    /// Strings without newlines, they are simply appended.
    /// Strings with newlines are appended and also new lines
    /// are indented based on the current indent level.
    pub fn push_str(&mut self, src: &str) {
        let lines = src.lines().collect::<Vec<_>>();
        let mut trim = None;
        for (i, line) in lines.iter().enumerate() {
            self.body.contents.push_str(if lines.len() == 1 {
                line
            } else {
                let trim = match trim {
                    Some(n) => n,
                    None => {
                        let val = line.len() - line.trim_start().len();
                        if !line.is_empty() {
                            trim = Some(val);
                        }
                        val
                    }
                };
                line.get(trim..).unwrap_or("")
            });
            if i != lines.len() - 1 || src.ends_with("\n") {
                self.newline();
            }
        }
    }

    /// Prints the documentation as comments
    /// e.g.
    /// > \# Line one of docs node
    /// >
    /// > \# Line two of docs node
    pub fn comment(&mut self, docs: &Docs) {
        let docs = match &docs.contents {
            Some(docs) => docs,
            None => return,
        };
        for line in docs.lines() {
            self.push_str(&format!("# {}\n", line));
        }
    }

    /// Prints the documentation as comments
    /// e.g.
    /// > """
    /// >
    /// > Line one of docs node
    /// >
    /// > Line two of docs node
    /// >
    /// > """
    pub fn docstring(&mut self, docs: &Docs) {
        let docs = match &docs.contents {
            Some(docs) => docs,
            None => return,
        };
        let triple_quote = r#"""""#;
        self.push_str(triple_quote);
        self.newline();
        for line in docs.lines() {
            self.push_str(line);
            self.newline();
        }
        self.push_str(triple_quote);
        self.newline();
    }

    /// Indent the source one level.
    pub fn indent(&mut self) {
        self.body.indent += 4;
        self.body.contents.push_str("    ");
    }

    /// Unindent, or in Python terms "dedent",
    /// the source one level.
    pub fn dedent(&mut self) {
        self.body.indent -= 4;
        assert!(self.body.contents.ends_with("    "));
        self.body.contents.pop();
        self.body.contents.pop();
        self.body.contents.pop();
        self.body.contents.pop();
    }

    /// Go to the next line and apply any indent.
    pub fn newline(&mut self) {
        self.body.contents.push_str("\n");
        for _ in 0..self.body.indent {
            self.body.contents.push_str(" ");
        }
    }

    pub fn pyimport<'a>(&mut self, module: &str, name: impl Into<Option<&'a str>>) {
        self.imports.pyimport(module, name.into())
    }

    pub fn finish(&self) -> String {
        let mut ret = self.imports.finish();
        ret.push_str(&self.body.contents);
        return ret;
    }

    pub fn is_empty(&self) -> bool {
        self.imports.is_empty() && self.body.contents.is_empty()
    }

    pub fn take_body(&mut self) -> Body {
        mem::take(&mut self.body)
    }

    pub fn replace_body(&mut self, body: Body) -> String {
        mem::replace(&mut self.body, body).contents
    }
}

impl Write for Source {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_append() {
        let mut s = Source::default();
        s.push_str("x");
        assert_eq!(s.body.contents, "x");
        s.push_str("y");
        assert_eq!(s.body.contents, "xy");
        s.push_str("z ");
        assert_eq!(s.body.contents, "xyz ");
        s.push_str(" a ");
        assert_eq!(s.body.contents, "xyz  a ");
        s.push_str("\na");
        assert_eq!(s.body.contents, "xyz  a \na");
    }

    #[test]
    fn trim_ws() {
        let mut s = Source::default();
        s.push_str("def foo():\n  return 1\n");
        assert_eq!(s.body.contents, "def foo():\n  return 1\n");
    }
}
