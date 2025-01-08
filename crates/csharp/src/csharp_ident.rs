use heck::{ToLowerCamelCase, ToUpperCamelCase};

pub(crate) trait ToCSharpIdent: ToOwned {
    fn csharp_keywords() -> &'static [&'static str];
    fn to_csharp_ident(&self) -> Self::Owned;
    fn to_csharp_ident_upper(&self) -> Self::Owned;
}

impl ToCSharpIdent for str {
    // Source: https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/keywords/
    fn csharp_keywords() -> &'static [&'static str] {
        static CSHARP_KEY_WORDS: &[&str] = &[
            "abstract",
            "as",
            "base",
            "bool",
            "break",
            "byte",
            "case",
            "catch",
            "char",
            "checked",
            "class",
            "const",
            "continue",
            "decimal",
            "default",
            "delegate",
            "do",
            "double",
            "else",
            "enum",
            "event",
            "explicit",
            "extern",
            "false",
            "finally",
            "fixed",
            "float",
            "for",
            "foreach",
            "goto",
            "if",
            "implicit",
            "in",
            "int",
            "interface",
            "internal",
            "is",
            "lock",
            "long",
            "namespace",
            "new",
            "null",
            "object",
            "operator",
            "out",
            "override",
            "params",
            "private",
            "protected",
            "public",
            "readonly",
            "ref",
            "return",
            "sbyte",
            "sealed",
            "short",
            "sizeof",
            "stackalloc",
            "static",
            "string",
            "struct",
            "switch",
            "this",
            "throw",
            "true",
            "try",
            "typeof",
            "uint",
            "ulong",
            "unchecked",
            "unsafe",
            "ushort",
            "using",
            "virtual",
            "void",
            "volatile",
            "while",
        ];
        CSHARP_KEY_WORDS
    }

    fn to_csharp_ident(&self) -> String {
        // Escape C# keywords
        if Self::csharp_keywords().contains(&self) {
            format!("@{}", self)
        } else {
            self.to_lower_camel_case()
        }
    }

    fn to_csharp_ident_upper(&self) -> String {
        // Escape C# keywords
        if Self::csharp_keywords().contains(&self) {
            format!("@{}", self)
        } else {
            self.to_upper_camel_case()
        }
    }
}
