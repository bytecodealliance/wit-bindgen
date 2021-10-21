use crate::{
    lex::{self, profile::Token, Span},
    Error,
};
use anyhow::{bail, Result};
use std::borrow::Cow;

type Tokenizer<'a> = lex::Tokenizer<'a, Token>;

pub struct Ast<'a> {
    pub items: Vec<Item<'a>>,
}

impl<'a> Ast<'a> {
    pub fn parse(input: &'a str) -> Result<Ast<'a>> {
        let mut lexer = Tokenizer::new(input);
        let mut items = Vec::new();

        while lexer.clone().next()?.is_some() {
            let docs = Docs::parse(&mut lexer)?;
            items.push(Item::parse(&mut lexer, docs)?);
        }

        Ok(Ast { items })
    }
}

pub enum Item<'a> {
    Extend(Extend<'a>),
    Provide(Provide<'a>),
    Require(Require<'a>),
    Implement(Implement<'a>),
}

impl<'a> Item<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Item<'a>> {
        match tokens.clone().next()? {
            Some((_span, Token::Extend)) => Extend::parse(tokens).map(Item::Extend),
            Some((_span, Token::Provide)) => Provide::parse(tokens, docs).map(Item::Provide),
            Some((_span, Token::Require)) => Require::parse(tokens, docs).map(Item::Require),
            Some((_span, Token::Implement)) => Implement::parse(tokens, docs).map(Item::Implement),
            other => {
                let (span, msg) = tokens
                    .format_expected_error("`extend`, `provide`, `require`, or `implement`", other);
                bail!(Error { span, msg })
            }
        }
    }
}

pub struct Docs<'a> {
    pub docs: Vec<Cow<'a, str>>,
}

impl<'a> Docs<'a> {
    fn parse(tokens: &mut Tokenizer<'a>) -> Result<Self> {
        let mut docs = Self { docs: Vec::new() };
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
}

pub struct Extend<'a> {
    pub span: Span,
    pub profile: Cow<'a, str>,
}

impl<'a> Extend<'a> {
    fn parse(tokens: &mut Tokenizer<'a>) -> Result<Self> {
        let mut span = tokens.expect(Token::Extend)?;
        let profile = tokens.expect(Token::StrLit)?;

        span.end = profile.end;

        Ok(Self {
            span,
            profile: tokens.parse_str(profile).into(),
        })
    }
}

pub struct Provide<'a> {
    pub docs: Docs<'a>,
    pub span: Span,
    pub interface: Cow<'a, str>,
}

impl<'a> Provide<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        let mut span = tokens.expect(Token::Provide)?;
        let interface = tokens.expect(Token::StrLit)?;

        span.end = interface.end;

        Ok(Self {
            docs,
            span,
            interface: tokens.parse_str(interface).into(),
        })
    }
}

pub struct Require<'a> {
    pub docs: Docs<'a>,
    pub span: Span,
    pub interface: Cow<'a, str>,
}

impl<'a> Require<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        let mut span = tokens.expect(Token::Require)?;
        let interface = tokens.expect(Token::StrLit)?;

        span.end = interface.end;

        Ok(Self {
            docs,
            span,
            interface: tokens.parse_str(interface).into(),
        })
    }
}

pub struct Implement<'a> {
    pub docs: Docs<'a>,
    pub span: Span,
    pub interface: Cow<'a, str>,
    pub component: Cow<'a, str>,
}

impl<'a> Implement<'a> {
    fn parse(tokens: &mut Tokenizer<'a>, docs: Docs<'a>) -> Result<Self> {
        let mut span = tokens.expect(Token::Implement)?;
        let interface = tokens.expect(Token::StrLit)?;
        tokens.expect(Token::With)?;
        let component = tokens.expect(Token::StrLit)?;

        span.end = component.end;

        Ok(Self {
            docs,
            span,
            interface: tokens.parse_str(interface).into(),
            component: tokens.parse_str(component).into(),
        })
    }
}
