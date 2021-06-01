use crate::Interface;
use anyhow::Result;
use lex::{Span, Token, Tokenizer};
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;

mod lex;
mod resolve;

pub struct Ast<'a> {
    pub items: Vec<Item<'a>>,
}

pub enum Item<'a> {
    Use(Use<'a>),
    Resource(Resource<'a>),
    TypeDef(TypeDef<'a>),
    Function(Function<'a>),
}

pub struct Id<'a> {
    pub name: Cow<'a, str>,
    pub span: Span,
}

impl<'a> From<&'a str> for Id<'a> {
    fn from(s: &'a str) -> Id<'a> {
        Id {
            name: s.into(),
            span: Span { start: 0, end: 0 },
        }
    }
}

impl<'a> From<String> for Id<'a> {
    fn from(s: String) -> Id<'a> {
        Id {
            name: s.into(),
            span: Span { start: 0, end: 0 },
        }
    }
}

pub struct Use<'a> {
    pub from: Vec<Id<'a>>,
    names: Option<Vec<UseName<'a>>>,
}

struct UseName<'a> {
    name: Id<'a>,
    as_: Option<Id<'a>>,
}

pub struct Resource<'a> {
    docs: Documentation<'a>,
    name: Id<'a>,
}

#[derive(Default)]
struct Documentation<'a> {
    docs: Vec<&'a str>,
}

pub struct TypeDef<'a> {
    docs: Documentation<'a>,
    name: Id<'a>,
    ty: Type<'a>,
}

enum Type<'a> {
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
    Handle(Id<'a>),
    Name(Id<'a>),
    List(Box<Type<'a>>),
    Record(Record<'a>),
    Variant(Variant<'a>),
    PushBuffer(Box<Type<'a>>),
    PullBuffer(Box<Type<'a>>),
}

struct Record<'a> {
    fields: Vec<Field<'a>>,
}

struct Field<'a> {
    docs: Documentation<'a>,
    name: Id<'a>,
    ty: Type<'a>,
}

struct Variant<'a> {
    span: Span,
    cases: Vec<Case<'a>>,
}

struct Case<'a> {
    docs: Documentation<'a>,
    name: Id<'a>,
    ty: Option<Type<'a>>,
}

pub struct Function<'a> {
    docs: Documentation<'a>,
    name: Id<'a>,
    params: Vec<(Id<'a>, Type<'a>)>,
    results: Vec<Type<'a>>,
}

impl<'a> Ast<'a> {
    pub fn parse(input: &'a str) -> Result<Ast<'a>> {
        let mut lexer = Tokenizer::new(input);
        let mut items = Vec::new();
        while lexer.clone().next()?.is_some() {
            items.push(Item::parse(&mut lexer)?);
        }
        Ok(Ast { items })
    }

    pub fn resolve(&self, map: &HashMap<String, Interface>) -> Result<Interface> {
        let mut resolver = resolve::Resolver::default();
        let instance = resolver.resolve(&self.items, map)?;
        Ok(instance)
    }
}

impl<'a> Item<'a> {
    fn parse(tokens: &mut Tokenizer<'a>) -> Result<Item<'a>> {
        let docs = parse_docs(tokens)?;
        match tokens.clone().next()? {
            Some((_span, Token::Use)) => Use::parse(tokens, docs).map(Item::Use),
            Some((_span, Token::Type)) => TypeDef::parse(tokens, docs).map(Item::TypeDef),
            Some((_span, Token::Flags)) => TypeDef::parse_flags(tokens, docs).map(Item::TypeDef),
            Some((_span, Token::Enum)) => TypeDef::parse_enum(tokens, docs).map(Item::TypeDef),
            Some((_span, Token::Variant)) => {
                TypeDef::parse_variant(tokens, docs).map(Item::TypeDef)
            }
            Some((_span, Token::Record)) => TypeDef::parse_record(tokens, docs).map(Item::TypeDef),
            Some((_span, Token::Union)) => TypeDef::parse_union(tokens, docs).map(Item::TypeDef),
            Some((_span, Token::Resource)) => Resource::parse(tokens, docs).map(Item::Resource),
            Some((_span, Token::Fn_)) => Function::parse(tokens, docs).map(Item::Function),
            other => Err(err_expected(tokens, "`type`, `resource`, or `fn`", other).into()),
        }
    }
}

impl<'a> Use<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, _docs: Documentation<'a>) -> Result<Self> {
        tokens.expect(Token::Use)?;
        let mut names = None;
        loop {
            if names.is_none() {
                if tokens.eat(Token::Star)? {
                    break;
                }
                tokens.expect(Token::LeftBrace)?;
                names = Some(Vec::new());
            }
            let names = names.as_mut().unwrap();
            let mut name = UseName {
                name: parse_id(tokens)?,
                as_: None,
            };
            if tokens.eat(Token::As)? {
                name.as_ = Some(parse_id(tokens)?);
            }
            names.push(name);
            if !tokens.eat(Token::Comma)? {
                break;
            }
        }
        if !names.is_none() {
            tokens.expect(Token::RightBrace)?;
        }
        tokens.expect(Token::From_)?;
        let mut from = vec![parse_id(tokens)?];
        while tokens.eat(Token::Colon)? {
            tokens.expect_raw(Token::Colon)?;
            from.push(parse_id(tokens)?);
        }
        Ok(Use { from, names })
    }
}

impl<'a> TypeDef<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, docs: Documentation<'a>) -> Result<Self> {
        tokens.expect(Token::Type)?;
        let name = parse_id(tokens)?;
        tokens.expect(Token::Equals)?;
        let ty = Type::parse(tokens)?;
        Ok(TypeDef { docs, name, ty })
    }

    fn parse_flags(tokens: &mut Tokenizer<'a>, docs: Documentation<'a>) -> Result<Self> {
        tokens.expect(Token::Flags)?;
        let name = parse_id(tokens)?;
        let ty = Type::Record(Record {
            fields: parse_list(
                tokens,
                Token::LeftBrace,
                Token::RightBrace,
                |docs, tokens| {
                    let name = parse_id(tokens)?;
                    Ok(Field {
                        docs,
                        name,
                        ty: Type::bool(),
                    })
                },
            )?,
        });
        Ok(TypeDef { docs, name, ty })
    }

    fn parse_record(tokens: &mut Tokenizer<'a>, docs: Documentation<'a>) -> Result<Self> {
        tokens.expect(Token::Record)?;
        let name = parse_id(tokens)?;
        let ty = Type::Record(Record {
            fields: parse_list(
                tokens,
                Token::LeftBrace,
                Token::RightBrace,
                |docs, tokens| {
                    let name = parse_id(tokens)?;
                    tokens.expect(Token::Colon)?;
                    let ty = Type::parse(tokens)?;
                    Ok(Field { docs, name, ty })
                },
            )?,
        });
        Ok(TypeDef { docs, name, ty })
    }

    fn parse_variant(tokens: &mut Tokenizer<'a>, docs: Documentation<'a>) -> Result<Self> {
        tokens.expect(Token::Variant)?;
        let name = parse_id(tokens)?;
        let ty = Type::Variant(Variant {
            span: name.span,
            cases: parse_list(
                tokens,
                Token::LeftBrace,
                Token::RightBrace,
                |docs, tokens| {
                    let name = parse_id(tokens)?;
                    let ty = if tokens.eat(Token::LeftParen)? {
                        let ty = Type::parse(tokens)?;
                        tokens.expect(Token::RightParen)?;
                        Some(ty)
                    } else {
                        None
                    };
                    Ok(Case { docs, name, ty })
                },
            )?,
        });
        Ok(TypeDef { docs, name, ty })
    }

    fn parse_union(tokens: &mut Tokenizer<'a>, docs: Documentation<'a>) -> Result<Self> {
        tokens.expect(Token::Union)?;
        let name = parse_id(tokens)?;
        let mut i = 0;
        let ty = Type::Variant(Variant {
            span: name.span,
            cases: parse_list(
                tokens,
                Token::LeftBrace,
                Token::RightBrace,
                |docs, tokens| {
                    let ty = Type::parse(tokens)?;
                    i += 1;
                    Ok(Case {
                        docs,
                        name: (i - 1).to_string().into(),
                        ty: Some(ty),
                    })
                },
            )?,
        });
        Ok(TypeDef { docs, name, ty })
    }

    fn parse_enum(tokens: &mut Tokenizer<'a>, docs: Documentation<'a>) -> Result<Self> {
        tokens.expect(Token::Enum)?;
        let name = parse_id(tokens)?;
        let ty = Type::Variant(Variant {
            span: name.span,
            cases: parse_list(
                tokens,
                Token::LeftBrace,
                Token::RightBrace,
                |docs, tokens| {
                    let name = parse_id(tokens)?;
                    Ok(Case {
                        docs,
                        name,
                        ty: None,
                    })
                },
            )?,
        });
        Ok(TypeDef { docs, name, ty })
    }
}

impl<'a> Resource<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, docs: Documentation<'a>) -> Result<Self> {
        tokens.expect(Token::Resource)?;
        let name = parse_id(tokens)?;
        Ok(Resource { docs, name })
    }
}

impl<'a> Function<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, docs: Documentation<'a>) -> Result<Self> {
        tokens.expect(Token::Fn_)?;
        let name = parse_id(tokens)?;
        let params = parse_list(
            tokens,
            Token::LeftParen,
            Token::RightParen,
            |_docs, tokens| {
                let name = parse_id(tokens)?;
                tokens.expect(Token::Colon)?;
                let ty = Type::parse(tokens)?;
                Ok((name, ty))
            },
        )?;
        let mut results = Vec::new();
        if tokens.eat(Token::RArrow)? {
            loop {
                results.push(Type::parse(tokens)?);
                if !tokens.eat(Token::Comma)? {
                    break;
                }
            }
        }
        Ok(Function {
            docs,
            name,
            params,
            results,
        })
    }
}

fn parse_id<'a>(tokens: &mut Tokenizer<'a>) -> Result<Id<'a>> {
    match tokens.next()? {
        Some((span, Token::Id)) => Ok(Id {
            name: tokens.get_span(span).into(),
            span,
        }),
        Some((span, Token::StrLit)) => Ok(Id {
            name: tokens.parse_str(span).into(),
            span,
        }),
        other => Err(err_expected(tokens, "an identifier or string", other).into()),
    }
}

fn parse_docs<'a>(tokens: &mut Tokenizer<'a>) -> Result<Documentation<'a>> {
    let mut docs = Documentation::default();
    let mut clone = tokens.clone();
    while let Some((span, token)) = clone.next_raw()? {
        match token {
            Token::Whitespace => {}
            Token::Comment => docs.docs.push(tokens.get_span(span)),
            _ => break,
        };
        *tokens = clone.clone();
    }
    Ok(docs)
}

impl<'a> Type<'a> {
    fn parse(tokens: &mut Tokenizer<'a>) -> Result<Self> {
        match tokens.next()? {
            Some((_span, Token::U8)) => Ok(Type::U8),
            Some((_span, Token::U16)) => Ok(Type::U16),
            Some((_span, Token::U32)) => Ok(Type::U32),
            Some((_span, Token::U64)) => Ok(Type::U64),
            Some((_span, Token::S8)) => Ok(Type::S8),
            Some((_span, Token::S16)) => Ok(Type::S16),
            Some((_span, Token::S32)) => Ok(Type::S32),
            Some((_span, Token::S64)) => Ok(Type::S64),
            Some((_span, Token::F32)) => Ok(Type::F32),
            Some((_span, Token::F64)) => Ok(Type::F64),
            Some((_span, Token::Char)) => Ok(Type::Char),
            Some((_span, Token::Handle)) => {
                let name = parse_id(tokens)?;
                Ok(Type::Handle(name))
            }

            // (...) -- tuples
            Some((_span, Token::LeftParen)) => {
                let mut fields = Vec::new();
                while !tokens.eat(Token::RightParen)? {
                    let field = Field {
                        docs: Documentation::default(),
                        name: fields.len().to_string().into(),
                        ty: Type::parse(tokens)?,
                    };
                    fields.push(field);
                    if !tokens.eat(Token::Comma)? {
                        tokens.expect(Token::RightParen)?;
                        break;
                    }
                }
                Ok(Type::Record(Record { fields }))
            }

            Some((_span, Token::Bool)) => Ok(Type::bool()),
            Some((_span, Token::String_)) => Ok(Type::List(Box::new(Type::Char))),

            // list<T>
            Some((_span, Token::List)) => {
                tokens.expect(Token::LessThan)?;
                let ty = Type::parse(tokens)?;
                tokens.expect(Token::GreaterThan)?;
                Ok(Type::List(Box::new(ty)))
            }

            // option<T>
            Some((span, Token::Option_)) => {
                tokens.expect(Token::LessThan)?;
                let ty = Type::parse(tokens)?;
                tokens.expect(Token::GreaterThan)?;
                Ok(Type::Variant(Variant {
                    span,
                    cases: vec![
                        Case {
                            docs: Documentation::default(),
                            name: "none".into(),
                            ty: None,
                        },
                        Case {
                            docs: Documentation::default(),
                            name: "some".into(),
                            ty: Some(ty),
                        },
                    ],
                }))
            }

            // expected<T, E>
            Some((span, Token::Expected)) => {
                tokens.expect(Token::LessThan)?;
                let ok = if tokens.eat(Token::Underscore)? {
                    None
                } else {
                    Some(Type::parse(tokens)?)
                };
                tokens.expect(Token::Comma)?;
                let err = if tokens.eat(Token::Underscore)? {
                    None
                } else {
                    Some(Type::parse(tokens)?)
                };
                tokens.expect(Token::GreaterThan)?;
                Ok(Type::Variant(Variant {
                    span,
                    cases: vec![
                        Case {
                            docs: Documentation::default(),
                            name: "ok".into(),
                            ty: ok,
                        },
                        Case {
                            docs: Documentation::default(),
                            name: "err".into(),
                            ty: err,
                        },
                    ],
                }))
            }

            // `foo`
            Some((span, Token::Id)) => Ok(Type::Name(Id {
                name: tokens.get_span(span).into(),
                span,
            })),
            // `"foo"`
            Some((span, Token::StrLit)) => Ok(Type::Name(Id {
                name: tokens.parse_str(span).into(),
                span,
            })),

            // push-buffer<T>
            Some((_span, Token::PushBuffer)) => {
                tokens.expect(Token::LessThan)?;
                let ty = Type::parse(tokens)?;
                tokens.expect(Token::GreaterThan)?;
                Ok(Type::PushBuffer(Box::new(ty)))
            }

            // pull-buffer<T>
            Some((_span, Token::PullBuffer)) => {
                tokens.expect(Token::LessThan)?;
                let ty = Type::parse(tokens)?;
                tokens.expect(Token::GreaterThan)?;
                Ok(Type::PullBuffer(Box::new(ty)))
            }

            other => Err(err_expected(tokens, "a type", other).into()),
        }
    }

    fn bool() -> Type<'static> {
        Type::Variant(Variant {
            span: Span { start: 0, end: 0 },
            cases: vec![
                Case {
                    docs: Documentation::default(),
                    name: "false".into(),
                    ty: None,
                },
                Case {
                    docs: Documentation::default(),
                    name: "true".into(),
                    ty: None,
                },
            ],
        })
    }
}

fn parse_list<'a, T>(
    tokens: &mut Tokenizer<'a>,
    start: Token,
    end: Token,
    mut parse: impl FnMut(Documentation<'a>, &mut Tokenizer<'a>) -> Result<T>,
) -> Result<Vec<T>> {
    tokens.expect(start)?;
    let mut items = Vec::new();
    loop {
        // get docs before we skip them to try to eat the end token
        let docs = parse_docs(tokens)?;

        // if we found an end token then we're done
        if tokens.eat(end)? {
            break;
        }

        let item = parse(docs, tokens)?;
        items.push(item);

        // if there's no trailing comma then this is required to be the end,
        // otherwise we go through the loop to try to get another item
        if !tokens.eat(Token::Comma)? {
            tokens.expect(end)?;
            break;
        }
    }
    Ok(items)
}

fn err_expected(
    tokens: &Tokenizer<'_>,
    expected: &'static str,
    found: Option<(Span, Token)>,
) -> Error {
    match found {
        Some((span, token)) => Error {
            span,
            msg: format!("expected {}, found {}", expected, token.describe()),
        },
        None => Error {
            span: Span {
                start: u32::try_from(tokens.input().len()).unwrap(),
                end: u32::try_from(tokens.input().len()).unwrap(),
            },
            msg: format!("expected {}, found eof", expected),
        },
    }
}

#[derive(Debug)]
struct Error {
    span: Span,
    msg: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.msg.fmt(f)
    }
}

impl std::error::Error for Error {}

pub fn rewrite_error(err: &mut anyhow::Error, file: &str, contents: &str) {
    let parse = match err.downcast_mut::<Error>() {
        Some(err) => err,
        None => return lex::rewrite_error(err, file, contents),
    };
    let msg = highlight_err(
        parse.span.start as usize,
        Some(parse.span.end as usize),
        file,
        contents,
        &parse.msg,
    );
    *err = anyhow::anyhow!("{}", msg);
}

fn highlight_err(
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
                msg.push_str("-");
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
