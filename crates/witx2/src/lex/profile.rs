use super::{is_keylike, Error, Tokenizer};

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum Token {
    Whitespace,
    Comment,
    StrLit,

    Extend,
    Provide,
    Require,
    Implement,
    With,

    Id,
}

impl super::Token for Token {
    fn whitespace() -> Self {
        Self::Whitespace
    }

    fn comment() -> Self {
        Self::Comment
    }

    fn string() -> Self {
        Self::StrLit
    }

    fn parse(start: usize, ch: char, tokenizer: &mut Tokenizer<'_, Self>) -> Result<Self, Error> {
        Ok(match ch {
            ch if is_keylike(ch) => {
                let consumed = tokenizer.eat_while(is_keylike);
                let end = start + ch.len_utf8() + consumed;
                match &tokenizer.input()[start..end] {
                    "extend" => Self::Extend,
                    "provide" => Self::Provide,
                    "require" => Self::Require,
                    "implement" => Self::Implement,
                    "with" => Self::With,
                    _ => Self::Id,
                }
            }
            _ => return Err(Error::Unexpected(start, ch)),
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
            Self::Id => "an identifier",
        }
    }
}
