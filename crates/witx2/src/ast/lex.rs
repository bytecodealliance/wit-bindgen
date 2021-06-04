use std::char;
use std::convert::TryFrom;
use std::fmt;
use std::str;

use self::Token::*;

#[derive(Clone)]
pub struct Tokenizer<'a> {
    input: &'a str,
    chars: CrlfFold<'a>,
}

#[derive(Clone)]
struct CrlfFold<'a> {
    chars: str::CharIndices<'a>,
}

/// A span, designating a range of bytes where a token is located.
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct Span {
    /// The start of the range.
    pub start: u32,
    /// The end of the range (exclusive).
    pub end: u32,
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum Token {
    Whitespace,
    Comment,

    Equals,
    Comma,
    Colon,
    Semicolon,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LessThan,
    GreaterThan,
    RArrow,
    Star,

    Use,
    Type,
    Resource,
    Function,
    U8,
    U16,
    U32,
    U64,
    S8,
    S16,
    S32,
    S64,
    F32,
    F64,
    Char,
    Handle,
    Record,
    Flags,
    Variant,
    Enum,
    Union,
    Bool,
    String_,
    Option_,
    Expected,
    List,
    Underscore,
    PushBuffer,
    PullBuffer,
    As,
    From_,
    Static,
    Interface,

    Id,
    StrLit,
}

#[derive(Eq, PartialEq, Debug)]
pub enum Error {
    InvalidCharInString(usize, char),
    InvalidEscape(usize, char),
    // InvalidHexEscape(usize, char),
    // InvalidEscapeValue(usize, u32),
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

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str) -> Tokenizer<'a> {
        let mut t = Tokenizer {
            input,
            chars: CrlfFold {
                chars: input.char_indices(),
            },
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
        let mut l = Tokenizer::new(s);
        assert!(matches!(l.chars.next(), Some((_, '"'))));
        while let Some(c) = l.eat_str_char(0).unwrap() {
            ret.push(c);
        }
        ret
    }

    pub fn next(&mut self) -> Result<Option<(Span, Token)>, Error> {
        loop {
            match self.next_raw()? {
                Some((_, Token::Whitespace)) | Some((_, Token::Comment)) => {}
                other => break Ok(other),
            }
        }
    }

    pub fn next_raw(&mut self) -> Result<Option<(Span, Token)>, Error> {
        let (start, ch) = match self.chars.next() {
            Some(pair) => pair,
            None => return Ok(None),
        };
        let token = match ch {
            '\n' | '\t' | ' ' => {
                // Eat all contiguous whitespace tokens
                while self.eatc(' ') || self.eatc('\t') || self.eatc('\n') {}
                Whitespace
            }
            '/' => {
                // Eat a line comment if it's `//...`
                if self.eatc('/') {
                    while let Some((_, ch)) = self.chars.next() {
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

                Comment
            }
            '=' => Equals,
            ',' => Comma,
            ':' => Colon,
            ';' => Semicolon,
            '(' => LeftParen,
            ')' => RightParen,
            '{' => LeftBrace,
            '}' => RightBrace,
            '<' => LessThan,
            '>' => GreaterThan,
            '*' => Star,
            '-' => {
                if self.eatc('>') {
                    RArrow
                } else {
                    return Err(Error::Unexpected(start, '-'));
                }
            }
            '"' => {
                while let Some(_ch) = self.eat_str_char(start)? {}
                StrLit
            }
            ch if is_keylike(ch) => {
                let mut end = start + ch.len_utf8();
                let mut iter = self.chars.clone();
                while let Some((i, ch)) = iter.next() {
                    if !is_keylike(ch) {
                        end = i;
                        break;
                    }
                    self.chars = iter.clone();
                }
                match &self.input[start..end] {
                    "use" => Use,
                    "type" => Type,
                    "resource" => Resource,
                    "function" => Function,
                    "u8" => U8,
                    "u16" => U16,
                    "u32" => U32,
                    "u64" => U64,
                    "s8" => S8,
                    "s16" => S16,
                    "s32" => S32,
                    "s64" => S64,
                    "f32" => F32,
                    "f64" => F64,
                    "char" => Char,
                    "handle" => Handle,
                    "record" => Record,
                    "flags" => Flags,
                    "variant" => Variant,
                    "enum" => Enum,
                    "union" => Union,
                    "bool" => Bool,
                    "string" => String_,
                    "option" => Option_,
                    "expected" => Expected,
                    "list" => List,
                    "_" => Underscore,
                    "push-buffer" => PushBuffer,
                    "pull-buffer" => PullBuffer,
                    "as" => As,
                    "from" => From_,
                    "static" => Static,
                    "interface" => Interface,
                    _ => Id,
                }
            }
            ch => return Err(Error::Unexpected(start, ch)),
        };
        let end = match self.chars.clone().next() {
            Some((i, _)) => i,
            None => self.input.len(),
        };

        let start = u32::try_from(start).unwrap();
        let end = u32::try_from(end).unwrap();
        Ok(Some((Span { start, end }, token)))
    }

    pub fn eat(&mut self, expected: Token) -> Result<bool, Error> {
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

    pub fn expect(&mut self, expected: Token) -> Result<Span, Error> {
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

    pub fn expect_raw(&mut self, expected: Token) -> Result<Span, Error> {
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

    fn eatc(&mut self, ch: char) -> bool {
        let mut iter = self.chars.clone();
        match iter.next() {
            Some((_, ch2)) if ch == ch2 => {
                self.chars = iter;
                true
            }
            _ => false,
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
            Some((_, ch))
                if ch == '\u{09}' || ('\u{20}' <= ch && ch <= '\u{10ffff}' && ch != '\u{7f}') =>
            {
                ch
            }
            Some((i, '\n')) => return Err(Error::NewlineInString(i)),
            Some((i, ch)) => return Err(Error::InvalidCharInString(i, ch)),
            None => return Err(Error::UnterminatedString(start)),
        };
        Ok(Some(ch))
    }
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

fn is_keylike(ch: char) -> bool {
    ('A' <= ch && ch <= 'Z')
        || ('a' <= ch && ch <= 'z')
        || ('0' <= ch && ch <= '9')
        || ch == '-'
        || ch == '_'
}

impl Token {
    pub fn describe(&self) -> &'static str {
        match self {
            Whitespace => "whitespace",
            Comment => "a comment",
            Equals => "'='",
            Comma => "','",
            Colon => "':'",
            Semicolon => "';'",
            LeftParen => "'('",
            RightParen => "')'",
            LeftBrace => "'{'",
            RightBrace => "'}'",
            LessThan => "'<'",
            GreaterThan => "'>'",
            Use => "keyword `use`",
            Type => "keyword `type`",
            Resource => "keyword `resource`",
            Function => "keyword `function`",
            U8 => "keyword `u8`",
            U16 => "keyword `u16`",
            U32 => "keyword `u32`",
            U64 => "keyword `u64`",
            S8 => "keyword `s8`",
            S16 => "keyword `s16`",
            S32 => "keyword `s32`",
            S64 => "keyword `s64`",
            F32 => "keyword `f32`",
            F64 => "keyword `f64`",
            Char => "keyword `char`",
            Handle => "keyword `handle`",
            Record => "keyword `record`",
            Flags => "keyword `flags`",
            Variant => "keyword `variant`",
            Enum => "keyword `enum`",
            Union => "keyword `union`",
            Bool => "keyword `bool`",
            String_ => "keyword `string`",
            Option_ => "keyword `option`",
            Expected => "keyword `expected`",
            List => "keyword `list`",
            Underscore => "keyword `_`",
            Id => "an identifier",
            StrLit => "a string",
            PushBuffer => "keyword `push-buffer`",
            PullBuffer => "keyword `pull-buffer`",
            RArrow => "`->`",
            Star => "`*`",
            As => "keyword `as`",
            From_ => "keyword `from`",
            Static => "keyword `static`",
            Interface => "keyword `interface`",
        }
    }
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
    let msg = super::highlight_err(pos, None, file, contents, lex);
    *err = anyhow::anyhow!("{}", msg);
}
