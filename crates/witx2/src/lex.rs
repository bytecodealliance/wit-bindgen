use std::char;
use std::convert::TryFrom;
use std::fmt;
use std::marker::PhantomData;
use std::str;

pub mod interface;
pub mod profile;

pub trait Token: Eq + PartialEq + Copy + Clone {
    fn whitespace() -> Self;
    fn comment() -> Self;
    fn string() -> Self;
    fn parse(start: usize, ch: char, tokenizer: &mut Tokenizer<'_, Self>) -> Result<Self, Error>;
    fn ignored(&self) -> bool;
    fn describe(&self) -> &'static str;
}

fn is_keylike(ch: char) -> bool {
    ch == '_'
        || ch == '-'
        || ('A'..='Z').contains(&ch)
        || ('a'..='z').contains(&ch)
        || ('0'..='9').contains(&ch)
}

#[derive(Clone)]
struct CrlfFold<'a> {
    chars: str::CharIndices<'a>,
}

impl<'a> Iterator for CrlfFold<'a> {
    type Item = (usize, char);

    fn next(&mut self) -> Option<(usize, char)> {
        self.chars.next().map(|(i, c)| {
            if c == '\r' {
                let mut attempt = self.chars.clone();
                if let Some((_, '\n')) = attempt.next() {
                    self.chars = attempt;
                    return (i, '\n');
                }
            }
            (i, c)
        })
    }
}

/// A span, designating a range of bytes where a token is located.
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct Span {
    /// The start of the range.
    pub start: u32,
    /// The end of the range (exclusive).
    pub end: u32,
}

#[derive(Eq, PartialEq, Debug)]
pub enum Error {
    InvalidCharInString(usize, char),
    InvalidEscape(usize, char),
    Unexpected(usize, char),
    UnterminatedComment(usize),
    UnterminatedString(usize),
    NewlineInString(usize),
    Wanted {
        at: usize,
        expected: &'static str,
        found: &'static str,
    },
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unexpected(_, ch) => write!(f, "unexpected character {:?}", ch),
            Error::UnterminatedComment(_) => write!(f, "unterminated block comment"),
            Error::Wanted {
                expected, found, ..
            } => write!(f, "expected {}, found {}", expected, found),
            Error::UnterminatedString(_) => write!(f, "unterminated string literal"),
            Error::NewlineInString(_) => write!(f, "newline in string literal"),
            Error::InvalidCharInString(_, ch) => write!(f, "invalid character in string {:?}", ch),
            Error::InvalidEscape(_, ch) => write!(f, "invalid escape in string {:?}", ch),
        }
    }
}

#[derive(Clone)]
pub struct Tokenizer<'a, T> {
    input: &'a str,
    chars: CrlfFold<'a>,
    _phantom: PhantomData<T>,
}

impl<'a, T> Tokenizer<'a, T>
where
    T: Token,
{
    pub fn new(input: &'a str) -> Self {
        let mut t = Self {
            input,
            chars: CrlfFold {
                chars: input.char_indices(),
            },
            _phantom: PhantomData,
        };
        // Eat utf-8 BOM
        t.eatc('\u{feff}');
        t
    }

    pub fn input(&self) -> &'a str {
        self.input
    }

    pub fn get_span(&self, span: Span) -> &'a str {
        &self.input[span.start as usize..span.end as usize]
    }

    pub fn parse_str(&self, span: Span) -> String {
        let mut ret = String::new();
        let s = self.get_span(span);
        let mut l = Self::new(s);
        assert!(matches!(l.chars.next(), Some((_, '"'))));
        while let Some(c) = l.eat_str_char(0).unwrap() {
            ret.push(c);
        }
        ret
    }

    pub fn next(&mut self) -> Result<Option<(Span, T)>, Error> {
        loop {
            let next = self.next_raw()?;
            if let Some((_, t)) = &next {
                if t.ignored() {
                    continue;
                }
            }

            return Ok(next);
        }
    }

    pub fn next_raw(&mut self) -> Result<Option<(Span, T)>, Error> {
        let (start, ch) = match self.chars.next() {
            Some(pair) => pair,
            None => return Ok(None),
        };
        let token = match ch {
            '\n' | '\t' | ' ' => {
                // Eat all contiguous whitespace tokens
                while self.eatc(' ') || self.eatc('\t') || self.eatc('\n') {}
                T::whitespace()
            }
            '/' => {
                // Eat a line comment if it's `//...`
                if self.eatc('/') {
                    for (_, ch) in &mut self.chars {
                        if ch == '\n' {
                            break;
                        }
                    }
                // eat a block comment if it's `/*...`
                } else if self.eatc('*') {
                    let mut depth = 1;
                    while depth > 0 {
                        let (_, ch) = match self.chars.next() {
                            Some(pair) => pair,
                            None => return Err(Error::UnterminatedComment(start)),
                        };
                        match ch {
                            '/' if self.eatc('*') => depth += 1,
                            '*' if self.eatc('/') => depth -= 1,
                            _ => {}
                        }
                    }
                } else {
                    return Err(Error::Unexpected(start, ch));
                }

                T::comment()
            }
            '"' => {
                while let Some(_ch) = self.eat_str_char(start)? {}
                T::string()
            }
            ch => T::parse(start, ch, self)?,
        };

        let end = match self.chars.clone().next() {
            Some((i, _)) => i,
            None => self.input.len(),
        };

        let start = u32::try_from(start).unwrap();
        let end = u32::try_from(end).unwrap();
        Ok(Some((Span { start, end }, token)))
    }

    pub fn eat(&mut self, expected: T) -> Result<bool, Error> {
        let mut other = self.clone();
        match other.next()? {
            Some((_span, found)) if expected == found => {
                *self = other;
                Ok(true)
            }
            Some(_) => Ok(false),
            None => Ok(false),
        }
    }

    pub fn expect(&mut self, expected: T) -> Result<Span, Error> {
        match self.next()? {
            Some((span, found)) => {
                if expected == found {
                    Ok(span)
                } else {
                    Err(Error::Wanted {
                        at: usize::try_from(span.start).unwrap(),
                        expected: expected.describe(),
                        found: found.describe(),
                    })
                }
            }
            None => Err(Error::Wanted {
                at: self.input.len(),
                expected: expected.describe(),
                found: "eof",
            }),
        }
    }

    pub fn expect_raw(&mut self, expected: T) -> Result<Span, Error> {
        match self.next_raw()? {
            Some((span, found)) => {
                if expected == found {
                    Ok(span)
                } else {
                    Err(Error::Wanted {
                        at: usize::try_from(span.start).unwrap(),
                        expected: expected.describe(),
                        found: found.describe(),
                    })
                }
            }
            None => Err(Error::Wanted {
                at: self.input.len(),
                expected: expected.describe(),
                found: "eof",
            }),
        }
    }

    pub fn eatc(&mut self, ch: char) -> bool {
        let mut iter = self.chars.clone();
        match iter.next() {
            Some((_, ch2)) if ch == ch2 => {
                self.chars = iter;
                true
            }
            _ => false,
        }
    }

    pub fn eat_while(&mut self, cond: impl Fn(char) -> bool) -> usize {
        let remaining = self.chars.chars.as_str().len();
        let mut iter = self.chars.clone();
        while let Some((_, ch)) = iter.next() {
            if !cond(ch) {
                break;
            }
            self.chars = iter.clone();
        }
        remaining - self.chars.chars.as_str().len()
    }

    pub fn format_expected_error(
        &self,
        expected: &str,
        found: Option<(Span, T)>,
    ) -> (Span, String) {
        match found {
            Some((span, token)) => (
                span,
                format!("expected {}, found {}", expected, token.describe()),
            ),
            None => (
                Span {
                    start: u32::try_from(self.input().len()).unwrap(),
                    end: u32::try_from(self.input().len()).unwrap(),
                },
                format!("expected {}, found eof", expected),
            ),
        }
    }

    fn eat_str_char(&mut self, start: usize) -> Result<Option<char>, Error> {
        let ch = match self.chars.next() {
            Some((_, '"')) => return Ok(None),
            Some((_, '\\')) => match self.chars.next() {
                Some((_, '"')) => '"',
                Some((_, '\'')) => ('\''),
                Some((_, 't')) => ('\t'),
                Some((_, 'n')) => ('\n'),
                Some((_, 'r')) => ('\r'),
                Some((_, '\\')) => ('\\'),
                Some((i, c)) => return Err(Error::InvalidEscape(i, c)),
                None => return Err(Error::UnterminatedString(start)),
            },
            Some((_, ch)) if ch == '\u{09}' || ('\u{20}'..='\u{10ffff}').contains(&ch) => ch,
            Some((i, '\n')) => return Err(Error::NewlineInString(i)),
            Some((i, ch)) => return Err(Error::InvalidCharInString(i, ch)),
            None => return Err(Error::UnterminatedString(start)),
        };
        Ok(Some(ch))
    }
}

pub fn rewrite_error(err: &mut anyhow::Error, file: &str, contents: &str) {
    let lex = match err.downcast_mut::<Error>() {
        Some(err) => err,
        None => return,
    };
    let pos = match lex {
        Error::Unexpected(at, _)
        | Error::UnterminatedComment(at)
        | Error::Wanted { at, .. }
        | Error::UnterminatedString(at)
        | Error::NewlineInString(at)
        | Error::InvalidCharInString(at, _)
        | Error::InvalidEscape(at, _) => *at,
    };

    let msg = highlight_err(pos, None, file, contents, lex);
    *err = anyhow::anyhow!("{}", msg);
}

pub fn highlight_err(
    start: usize,
    end: Option<usize>,
    file: &str,
    input: &str,
    err: impl fmt::Display,
) -> String {
    let (line, col) = linecol_in(start, input);
    let snippet = input.lines().nth(line).unwrap_or("");
    let mut msg = format!(
        "\
{err}
     --> {file}:{line}:{col}
      |
 {line:4} | {snippet}
      | {marker:>0$}",
        col + 1,
        file = file,
        line = line + 1,
        col = col + 1,
        err = err,
        snippet = snippet,
        marker = "^",
    );
    if let Some(end) = end {
        if let Some(s) = input.get(start..end) {
            for _ in s.chars().skip(1) {
                msg.push('-');
            }
        }
    }
    return msg;

    fn linecol_in(pos: usize, text: &str) -> (usize, usize) {
        let mut cur = 0;
        // Use split_terminator instead of lines so that if there is a `\r`,
        // it is included in the offset calculation. The `+1` values below
        // account for the `\n`.
        for (i, line) in text.split_terminator('\n').enumerate() {
            if cur + line.len() + 1 > pos {
                return (i, pos - cur);
            }
            cur += line.len() + 1;
        }
        (text.lines().count(), 0)
    }
}
