use crate::abi::Abi;
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
    Value(Value<'a>),
    Interface(Interface<'a>),
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
    docs: Docs<'a>,
    name: Id<'a>,
    values: Vec<(bool, Value<'a>)>,
}

#[derive(Default)]
struct Docs<'a> {
    docs: Vec<Cow<'a, str>>,
}

pub struct TypeDef<'a> {
    docs: Docs<'a>,
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
    #[allow(dead_code)]
    Usize,
    #[allow(dead_code)]
    CChar,
    Handle(Id<'a>),
    Name(Id<'a>),
    List(Box<Type<'a>>),
    Record(Record<'a>),
    Variant(Variant<'a>),
    PushBuffer(Box<Type<'a>>),
    PullBuffer(Box<Type<'a>>),
    #[allow(dead_code)]
    Pointer(Box<Type<'a>>),
    #[allow(dead_code)]
    ConstPointer(Box<Type<'a>>),
}

struct Record<'a> {
    tuple_hint: bool,
    flags_repr: Option<Box<Type<'a>>>,
    fields: Vec<Field<'a>>,
}

struct Field<'a> {
    docs: Docs<'a>,
    name: Id<'a>,
    ty: Type<'a>,
}

struct Variant<'a> {
    tag: Option<Box<Type<'a>>>,
    span: Span,
    cases: Vec<Case<'a>>,
}

struct Case<'a> {
    docs: Docs<'a>,
    name: Id<'a>,
    ty: Option<Type<'a>>,
}

pub struct Value<'a> {
    docs: Docs<'a>,
    name: Id<'a>,
    kind: ValueKind<'a>,
}

enum ValueKind<'a> {
    Function {
        abi: crate::abi::Abi,
        params: Vec<(Id<'a>, Type<'a>)>,
        results: Vec<Type<'a>>,
    },
    Global(Type<'a>),
}

#[allow(dead_code)] // TODO
pub struct Interface<'a> {
    docs: Docs<'a>,
    name: Id<'a>,
    items: Vec<Item<'a>>,
}

impl<'a> Ast<'a> {
    pub fn parse(input: &'a str) -> Result<Ast<'a>> {
        let mut lexer = Tokenizer::new(input);
        #[cfg(feature = "old-witx-compat")]
        if lexer.eat(Token::Semicolon)? || lexer.eat(Token::LeftParen)? {
            return Ast::parse_old_witx(input);
        }
        let mut items = Vec::new();
        while lexer.clone().next()?.is_some() {
            let docs = parse_docs(&mut lexer)?;
            items.push(Item::parse(&mut lexer, docs)?);
        }
        Ok(Ast { items })
    }

    pub fn resolve(
        &self,
        name: &str,
        map: &HashMap<String, crate::Interface>,
    ) -> Result<crate::Interface> {
        let mut resolver = resolve::Resolver::default();
        let instance = resolver.resolve(name, &self.items, map)?;
        Ok(instance)
    }

    #[cfg(feature = "old-witx-compat")]
    fn parse_old_witx(input: &'a str) -> Result<Ast<'a>> {
        use witx::parser as old;
        let buf = wast::parser::ParseBuffer::new(&input)?;
        let doc = wast::parser::parse::<old::TopLevelModule>(&buf)?;
        let mut items = Vec::new();
        for d in doc.decls {
            let item = match d.item {
                old::TopLevelSyntax::Use(u) => Item::Use(Use {
                    from: vec![id(&u.from)],
                    names: match u.names {
                        old::UsedNames::All(_) => None,
                        old::UsedNames::List(names) => Some(
                            names
                                .iter()
                                .map(|n| UseName {
                                    name: id(&n.other_name),
                                    as_: Some(id(&n.our_name)),
                                })
                                .collect(),
                        ),
                    },
                }),
                old::TopLevelSyntax::Decl(u) => match u {
                    old::DeclSyntax::Typename(t) => Item::TypeDef(TypeDef {
                        docs: docs(&d.comments),
                        name: id(&t.ident),
                        ty: ty(&t.def),
                    }),
                    old::DeclSyntax::Resource(r) => Item::Resource(Resource {
                        docs: docs(&d.comments),
                        name: id(&r.ident),
                        values: Vec::new(),
                    }),
                    old::DeclSyntax::Const(_) => unimplemented!(),
                },
            };
            items.push(item);
        }

        for f in doc.functions {
            let item = Item::Value(Value {
                docs: docs(&f.comments),
                name: Id {
                    name: f.item.export.to_string().into(),
                    span: span(f.item.export_loc),
                },
                kind: ValueKind::Function {
                    abi: match f.item.abi {
                        witx::Abi::Next => Abi::Canonical,
                        witx::Abi::Preview1 => Abi::Preview1,
                    },
                    params: f
                        .item
                        .params
                        .iter()
                        .map(|p| (id(&p.item.name), ty(&p.item.type_)))
                        .collect(),
                    results: f.item.results.iter().map(|p| ty(&p.item.type_)).collect(),
                },
            });
            items.push(item);
        }

        return Ok(Ast { items });

        fn ty(t: &old::TypedefSyntax<'_>) -> Type<'static> {
            match t {
                old::TypedefSyntax::Record(e) => Type::Record(Record {
                    tuple_hint: false,
                    flags_repr: None,
                    fields: e
                        .fields
                        .iter()
                        .map(|f| Field {
                            docs: docs(&f.comments),
                            name: id(&f.item.name),
                            ty: ty(&f.item.type_),
                        })
                        .collect(),
                }),
                old::TypedefSyntax::Flags(e) => Type::Record(Record {
                    tuple_hint: false,
                    flags_repr: e.repr.as_ref().map(|t| Box::new(builtin(t))),
                    fields: e
                        .flags
                        .iter()
                        .map(|f| Field {
                            docs: docs(&f.comments),
                            name: id(&f.item),
                            ty: Type::bool(),
                        })
                        .collect(),
                }),
                old::TypedefSyntax::Tuple(e) => Type::Record(Record {
                    tuple_hint: true,
                    flags_repr: None,
                    fields: e
                        .types
                        .iter()
                        .enumerate()
                        .map(|(i, t)| Field {
                            docs: Docs::default(),
                            name: Id::from(i.to_string()),
                            ty: ty(t),
                        })
                        .collect(),
                }),

                old::TypedefSyntax::Variant(e) => Type::Variant(Variant {
                    tag: e.tag.as_ref().map(|t| Box::new(ty(t))),
                    span: Span { start: 0, end: 0 },
                    cases: e
                        .cases
                        .iter()
                        .map(|c| Case {
                            docs: docs(&c.comments),
                            name: id(&c.item.name),
                            ty: c.item.ty.as_ref().map(ty),
                        })
                        .collect(),
                }),
                old::TypedefSyntax::Enum(e) => Type::Variant(Variant {
                    tag: e.repr.as_ref().map(|t| Box::new(builtin(t))),
                    span: Span { start: 0, end: 0 },
                    cases: e
                        .members
                        .iter()
                        .map(|c| Case {
                            docs: docs(&c.comments),
                            name: id(&c.item),
                            ty: None,
                        })
                        .collect(),
                }),
                old::TypedefSyntax::Expected(e) => Type::Variant(Variant {
                    tag: None,
                    span: Span { start: 0, end: 0 },
                    cases: vec![
                        Case {
                            docs: Docs::default(),
                            name: "ok".into(),
                            ty: e.ok.as_ref().map(|t| ty(t)),
                        },
                        Case {
                            docs: Docs::default(),
                            name: "err".into(),
                            ty: e.err.as_ref().map(|t| ty(t)),
                        },
                    ],
                }),
                old::TypedefSyntax::Option(e) => Type::Variant(Variant {
                    tag: None,
                    span: Span { start: 0, end: 0 },
                    cases: vec![
                        Case {
                            docs: Docs::default(),
                            name: "none".into(),
                            ty: None,
                        },
                        Case {
                            docs: Docs::default(),
                            name: "some".into(),
                            ty: Some(ty(&e.ty)),
                        },
                    ],
                }),
                old::TypedefSyntax::Union(e) => Type::Variant(Variant {
                    tag: e.tag.as_ref().map(|t| Box::new(ty(t))),
                    span: Span { start: 0, end: 0 },
                    cases: e
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(i, c)| Case {
                            docs: docs(&c.comments),
                            name: i.to_string().into(),
                            ty: Some(ty(&c.item)),
                        })
                        .collect(),
                }),

                old::TypedefSyntax::Handle(e) => Type::Handle(id(&e.resource)),
                old::TypedefSyntax::List(e) => Type::List(Box::new(ty(e))),
                old::TypedefSyntax::Pointer(e) => Type::Pointer(Box::new(ty(e))),
                old::TypedefSyntax::ConstPointer(e) => Type::ConstPointer(Box::new(ty(e))),
                old::TypedefSyntax::Buffer(e) => {
                    if e.out {
                        Type::PushBuffer(Box::new(ty(&e.ty)))
                    } else {
                        Type::PullBuffer(Box::new(ty(&e.ty)))
                    }
                }
                old::TypedefSyntax::Builtin(e) => builtin(e),
                old::TypedefSyntax::Ident(e) => Type::Name(id(e)),
                old::TypedefSyntax::String => Type::List(Box::new(Type::Char)),
                old::TypedefSyntax::Bool => Type::bool(),
            }
        }

        fn builtin(e: &witx::BuiltinType) -> Type<'static> {
            use witx::BuiltinType::*;
            match e {
                Char => Type::Char,
                U8 { lang_c_char: false } => Type::U8,
                U8 { lang_c_char: true } => Type::CChar,
                S8 => Type::S8,
                U16 => Type::U16,
                S16 => Type::S16,
                U32 {
                    lang_ptr_size: false,
                } => Type::U32,
                U32 {
                    lang_ptr_size: true,
                } => Type::Usize,
                S32 => Type::S32,
                U64 => Type::U64,
                S64 => Type::S64,
                F32 => Type::F32,
                F64 => Type::F64,
            }
        }

        fn docs(docs: &old::CommentSyntax<'_>) -> Docs<'static> {
            Docs {
                docs: docs
                    .comments
                    .iter()
                    .map(|c| format!("// {}", c).into())
                    .collect(),
            }
        }

        fn id(id: &wast::Id<'_>) -> Id<'static> {
            Id {
                name: id.name().to_string().into(),
                span: span(id.span()),
            }
        }

        // TODO: should add an `offset` accessor to `wast::Span` upstream...
        fn span(span: wast::Span) -> Span {
            let mut low = 0;
            let mut high = 1024;
            while span > wast::Span::from_offset(high) {
                high *= 2;
            }
            while low != high {
                let val = (high + low) / 2;
                let mid = wast::Span::from_offset(val);
                if span < mid {
                    high = val - 1;
                } else if span > mid {
                    low = val + 1;
                } else {
                    low = val;
                    high = val;
                }
            }
            let low = low as u32;
            Span {
                start: low,
                end: low + 1,
            }
        }
    }
}

impl<'a> Item<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Item<'a>> {
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
            Some((_span, Token::Interface)) => Interface::parse(tokens, docs).map(Item::Interface),
            Some((_span, Token::Id)) | Some((_span, Token::StrLit)) => {
                Value::parse(tokens, docs).map(Item::Value)
            }
            other => Err(err_expected(tokens, "`type`, `resource`, or `fn`", other).into()),
        }
    }
}

impl<'a> Use<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, _docs: Docs<'a>) -> Result<Self> {
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
    fn parse(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Type)?;
        let name = parse_id(tokens)?;
        tokens.expect(Token::Equals)?;
        let ty = Type::parse(tokens)?;
        Ok(TypeDef { docs, name, ty })
    }

    fn parse_flags(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Flags)?;
        let name = parse_id(tokens)?;
        let ty = Type::Record(Record {
            flags_repr: None,
            tuple_hint: false,
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

    fn parse_record(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Record)?;
        let name = parse_id(tokens)?;
        let ty = Type::Record(Record {
            flags_repr: None,
            tuple_hint: false,
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

    fn parse_variant(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Variant)?;
        let name = parse_id(tokens)?;
        let ty = Type::Variant(Variant {
            tag: None,
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

    fn parse_union(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Union)?;
        let name = parse_id(tokens)?;
        let mut i = 0;
        let ty = Type::Variant(Variant {
            tag: None,
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

    fn parse_enum(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Enum)?;
        let name = parse_id(tokens)?;
        let ty = Type::Variant(Variant {
            tag: None,
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
    fn parse(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Resource)?;
        let name = parse_id(tokens)?;
        let mut values = Vec::new();
        if tokens.eat(Token::LeftBrace)? {
            loop {
                let docs = parse_docs(tokens)?;
                if tokens.eat(Token::RightBrace)? {
                    break;
                }
                let statik = tokens.eat(Token::Static)?;
                values.push((statik, Value::parse(tokens, docs)?));
            }
        }
        Ok(Resource { docs, name, values })
    }
}

impl<'a> Value<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        let name = parse_id(tokens)?;
        tokens.expect(Token::Colon)?;

        let kind = if tokens.eat(Token::Function)? {
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
            ValueKind::Function {
                abi: Abi::Canonical, // TODO
                params,
                results,
            }
        } else {
            ValueKind::Global(Type::parse(tokens)?)
        };
        Ok(Value { docs, name, kind })
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

fn parse_docs<'a>(tokens: &mut Tokenizer<'a>) -> Result<Docs<'a>> {
    let mut docs = Docs::default();
    let mut clone = tokens.clone();
    while let Some((span, token)) = clone.next_raw()? {
        match token {
            Token::Whitespace => {}
            Token::Comment => docs.docs.push(tokens.get_span(span).into()),
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
                        docs: Docs::default(),
                        name: fields.len().to_string().into(),
                        ty: Type::parse(tokens)?,
                    };
                    fields.push(field);
                    if !tokens.eat(Token::Comma)? {
                        tokens.expect(Token::RightParen)?;
                        break;
                    }
                }
                Ok(Type::Record(Record {
                    fields,
                    flags_repr: None,
                    tuple_hint: true,
                }))
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
                    tag: None,
                    span,
                    cases: vec![
                        Case {
                            docs: Docs::default(),
                            name: "none".into(),
                            ty: None,
                        },
                        Case {
                            docs: Docs::default(),
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
                    tag: None,
                    span,
                    cases: vec![
                        Case {
                            docs: Docs::default(),
                            name: "ok".into(),
                            ty: ok,
                        },
                        Case {
                            docs: Docs::default(),
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
            tag: None,
            span: Span { start: 0, end: 0 },
            cases: vec![
                Case {
                    docs: Docs::default(),
                    name: "false".into(),
                    ty: None,
                },
                Case {
                    docs: Docs::default(),
                    name: "true".into(),
                    ty: None,
                },
            ],
        })
    }
}

impl<'a> Interface<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Interface)?;
        let name = parse_id(tokens)?;
        tokens.expect(Token::LeftBrace)?;
        let mut items = Vec::new();
        loop {
            let docs = parse_docs(tokens)?;
            if tokens.eat(Token::RightBrace)? {
                break;
            }
            items.push(Item::parse(tokens, docs)?);
        }
        Ok(Interface { docs, name, items })
    }
}

fn parse_list<'a, T>(
    tokens: &mut Tokenizer<'a>,
    start: Token,
    end: Token,
    mut parse: impl FnMut(Docs<'a>, &mut Tokenizer<'a>) -> Result<T>,
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
