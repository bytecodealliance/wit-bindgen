// Code copied from jco

use heck::ToLowerCamelCase;
use wit_bindgen_core::wit_parser::{Resolve, Type, TypeDefKind};

/// Tests whether `ty` can be represented with `null`, and if it can then
/// the "other type" is returned. If `Some` is returned that means that `ty`
/// is `null | <return>`. If `None` is returned that means that `null` can't
/// be used to represent `ty`.
pub fn as_nullable<'a>(resolve: &'a Resolve, ty: &'a Type) -> Option<&'a Type> {
    let id = match ty {
        Type::Id(id) => *id,
        _ => return None,
    };
    match &resolve.types[id].kind {
        // If `ty` points to an `option<T>`, then `ty` can be represented
        // with `null` if `t` itself can't be represented with null. For
        // example `option<option<u32>>` can't be represented with `null`
        // since that's ambiguous if it's `none` or `some(none)`.
        //
        // Note, oddly enough, that `option<option<option<u32>>>` can be
        // represented as `null` since:
        //
        // * `null` => `none`
        // * `{ tag: "none" }` => `some(none)`
        // * `{ tag: "some", val: null }` => `some(some(none))`
        // * `{ tag: "some", val: 1 }` => `some(some(some(1)))`
        //
        // It's doubtful anyone would actually rely on that though due to
        // how confusing it is.
        TypeDefKind::Option(t) => {
            if !maybe_null(resolve, t) {
                Some(t)
            } else {
                None
            }
        }
        TypeDefKind::Type(t) => as_nullable(resolve, t),
        _ => None,
    }
}

pub fn maybe_null(resolve: &Resolve, ty: &Type) -> bool {
    as_nullable(resolve, ty).is_some()
}

// Convert an arbitrary string to a similar close js identifier
pub fn to_js_identifier(goal_name: &str) -> String {
    if is_js_identifier(goal_name) {
        goal_name.to_string()
    } else {
        let goal = goal_name.to_lower_camel_case();
        let mut identifier = String::new();
        for char in goal.chars() {
            let valid_char = if identifier.is_empty() {
                is_js_identifier_start(char)
            } else {
                is_js_identifier_char(char)
            };
            if valid_char {
                identifier.push(char);
            } else {
                identifier.push(match char {
                    '.' => '_',
                    _ => '$',
                });
            }
        }
        if !is_js_identifier(&identifier) {
            identifier = format!("_{identifier}");
            if !is_js_identifier(&identifier) {
                panic!("Unable to generate valid identifier {identifier} for '{goal_name}'");
            }
        }
        identifier
    }
}

pub fn is_js_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    if let Some(char) = chars.next() {
        if !is_js_identifier_start(char) {
            return false;
        }
    } else {
        return false;
    }
    for char in chars {
        if !is_js_identifier_char(char) {
            return false;
        }
    }
    !is_js_reserved_word(&s)
}

pub fn is_js_reserved_word(s: &str) -> bool {
    RESERVED_KEYWORDS.binary_search(&s).is_ok()
}

// https://tc39.es/ecma262/#prod-IdentifierStartChar
// Unicode ID_Start | "$" | "_"
fn is_js_identifier_start(code: char) -> bool {
    match code {
        'A'..='Z' | 'a'..='z' | '$' | '_' => true,
        // leaving out non-ascii for now...
        _ => false,
    }
}

// https://tc39.es/ecma262/#prod-IdentifierPartChar
// Unicode ID_Continue | "$" | U+200C | U+200D
fn is_js_identifier_char(code: char) -> bool {
    match code {
        '0'..='9' | 'A'..='Z' | 'a'..='z' | '$' | '_' => true,
        // leaving out non-ascii for now...
        _ => false,
    }
}

pub(crate) const RESERVED_KEYWORDS: &[&str] = &[
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "eval",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
];
