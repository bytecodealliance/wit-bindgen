use super::{is_keylike, Error, Tokenizer};

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
    String,
    Option,
    Expected,
    List,
    Underscore,
    PushBuffer,
    PullBuffer,
    As,
    From,
    Static,
    Interface,
    Tuple,
    Async,

    Id,
    StrLit,
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
            '=' => Self::Equals,
            ',' => Self::Comma,
            ':' => Self::Colon,
            ';' => Self::Semicolon,
            '(' => Self::LeftParen,
            ')' => Self::RightParen,
            '{' => Self::LeftBrace,
            '}' => Self::RightBrace,
            '<' => Self::LessThan,
            '>' => Self::GreaterThan,
            '*' => Self::Star,
            '-' => {
                if tokenizer.eatc('>') {
                    Self::RArrow
                } else {
                    return Err(Error::Unexpected(start, '-'));
                }
            }
            ch if is_keylike(ch) => {
                let consumed = tokenizer.eat_while(is_keylike);
                let end = start + ch.len_utf8() + consumed;
                match &tokenizer.input()[start..end] {
                    "use" => Self::Use,
                    "type" => Self::Type,
                    "resource" => Self::Resource,
                    "function" => Self::Function,
                    "u8" => Self::U8,
                    "u16" => Self::U16,
                    "u32" => Self::U32,
                    "u64" => Self::U64,
                    "s8" => Self::S8,
                    "s16" => Self::S16,
                    "s32" => Self::S32,
                    "s64" => Self::S64,
                    "f32" => Self::F32,
                    "f64" => Self::F64,
                    "char" => Self::Char,
                    "handle" => Self::Handle,
                    "record" => Self::Record,
                    "flags" => Self::Flags,
                    "variant" => Self::Variant,
                    "enum" => Self::Enum,
                    "union" => Self::Union,
                    "bool" => Self::Bool,
                    "string" => Self::String,
                    "option" => Self::Option,
                    "expected" => Self::Expected,
                    "list" => Self::List,
                    "_" => Self::Underscore,
                    "push-buffer" => Self::PushBuffer,
                    "pull-buffer" => Self::PullBuffer,
                    "as" => Self::As,
                    "from" => Self::From,
                    "static" => Self::Static,
                    "interface" => Self::Interface,
                    "tuple" => Self::Tuple,
                    "async" => Self::Async,
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
            Self::Equals => "'='",
            Self::Comma => "','",
            Self::Colon => "':'",
            Self::Semicolon => "';'",
            Self::LeftParen => "'('",
            Self::RightParen => "')'",
            Self::LeftBrace => "'{'",
            Self::RightBrace => "'}'",
            Self::LessThan => "'<'",
            Self::GreaterThan => "'>'",
            Self::Use => "keyword `use`",
            Self::Type => "keyword `type`",
            Self::Resource => "keyword `resource`",
            Self::Function => "keyword `function`",
            Self::U8 => "keyword `u8`",
            Self::U16 => "keyword `u16`",
            Self::U32 => "keyword `u32`",
            Self::U64 => "keyword `u64`",
            Self::S8 => "keyword `s8`",
            Self::S16 => "keyword `s16`",
            Self::S32 => "keyword `s32`",
            Self::S64 => "keyword `s64`",
            Self::F32 => "keyword `f32`",
            Self::F64 => "keyword `f64`",
            Self::Char => "keyword `char`",
            Self::Handle => "keyword `handle`",
            Self::Record => "keyword `record`",
            Self::Flags => "keyword `flags`",
            Self::Variant => "keyword `variant`",
            Self::Enum => "keyword `enum`",
            Self::Union => "keyword `union`",
            Self::Bool => "keyword `bool`",
            Self::String => "keyword `string`",
            Self::Option => "keyword `option`",
            Self::Expected => "keyword `expected`",
            Self::List => "keyword `list`",
            Self::Underscore => "keyword `_`",
            Self::Id => "an identifier",
            Self::StrLit => "a string literal",
            Self::PushBuffer => "keyword `push-buffer`",
            Self::PullBuffer => "keyword `pull-buffer`",
            Self::RArrow => "`->`",
            Self::Star => "`*`",
            Self::As => "keyword `as`",
            Self::From => "keyword `from`",
            Self::Static => "keyword `static`",
            Self::Interface => "keyword `interface`",
            Self::Tuple => "keyword `tuple`",
            Self::Async => "keyword `async`",
        }
    }
}
