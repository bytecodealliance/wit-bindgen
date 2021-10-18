use crate::{
    lex::{self, Tokenizer},
    Error,
};
use anyhow::{bail, Result};
use std::borrow::Cow;

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
enum Token {
    Whitespace,
    Comment,
    StrLit,

    Extend,
    Provide,
    Require,
    Implement,
    With,
}

impl Token {
    fn is_keyword_char(ch: char) -> bool {
        ('a'..='z').contains(&ch)
    }
}

impl lex::Token for Token {
    fn whitespace() -> Self {
        Self::Whitespace
    }

    fn comment() -> Self {
        Self::Comment
    }

    fn string() -> Self {
        Self::StrLit
    }

    fn parse(
        start: usize,
        ch: char,
        tokenizer: &mut Tokenizer<'_, Self>,
    ) -> Result<Self, lex::Error> {
        Ok(match ch {
            ch if Self::is_keyword_char(ch) => {
                let consumed = tokenizer.eat_while(Self::is_keyword_char);
                let end = start + ch.len_utf8() + consumed;
                match &tokenizer.input()[start..end] {
                    "extend" => Self::Extend,
                    "provide" => Self::Provide,
                    "require" => Self::Require,
                    "implement" => Self::Implement,
                    "with" => Self::With,
                    _ => return Err(lex::Error::Unexpected(start, ch)),
                }
            }
            _ => return Err(lex::Error::Unexpected(start, ch)),
        })
    }

    fn ignored(&self) -> bool {
        matches!(self, Self::Whitespace | Self::Comment)
    }

    fn describe(&self) -> &'static str {
        match self {
            Self::Whitespace => "whitespace",
            Self::Comment => "a comment",
            Self::StrLit => "a string literal",
            Token::Extend => "keyword `extend`",
            Token::Provide => "keyword `provide`",
            Token::Require => "keyword `require`",
            Token::Implement => "keyword `implement`",
            Token::With => "keyword `with`",
        }
    }
}

pub struct Ast<'a> {
    pub items: Vec<Item<'a>>,
}

impl<'a> Ast<'a> {
    pub fn parse(input: &'a str) -> Result<Ast<'a>> {
        let mut lexer = Tokenizer::<'a, Token>::new(input);
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
    fn parse(tokens: &mut Tokenizer<'a, Token>, docs: Docs<'a>) -> Result<Item<'a>> {
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
    fn parse(tokens: &mut Tokenizer<'a, Token>) -> Result<Self> {
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
    pub profile: Cow<'a, str>,
}

impl<'a> Extend<'a> {
    fn parse(tokens: &mut Tokenizer<'a, Token>) -> Result<Self> {
        tokens.expect(Token::Extend)?;

        let profile = tokens.expect(Token::StrLit)?;

        Ok(Self {
            profile: tokens.get_span(profile).into(),
        })
    }
}

pub struct Provide<'a> {
    pub docs: Docs<'a>,
    pub interface: Cow<'a, str>,
}

impl<'a> Provide<'a> {
    fn parse(tokens: &mut Tokenizer<'a, Token>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Provide)?;

        let interface = tokens.expect(Token::StrLit)?;

        Ok(Self {
            docs,
            interface: tokens.get_span(interface).into(),
        })
    }
}

pub struct Require<'a> {
    pub docs: Docs<'a>,
    pub interface: Cow<'a, str>,
}

impl<'a> Require<'a> {
    fn parse(tokens: &mut Tokenizer<'a, Token>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Require)?;

        let interface = tokens.expect(Token::StrLit)?;

        Ok(Self {
            docs,
            interface: tokens.get_span(interface).into(),
        })
    }
}

pub struct Implement<'a> {
    pub docs: Docs<'a>,
    pub interface: Cow<'a, str>,
    pub component: Cow<'a, str>,
}

impl<'a> Implement<'a> {
    fn parse(tokens: &mut Tokenizer<'a, Token>, docs: Docs<'a>) -> Result<Self> {
        tokens.expect(Token::Implement)?;

        let interface = tokens.expect(Token::StrLit)?;

        tokens.expect(Token::With)?;

        let implementation = tokens.expect(Token::StrLit)?;

        Ok(Self {
            docs,
            interface: tokens.get_span(interface).into(),
            component: tokens.get_span(implementation).into(),
        })
    }
}
