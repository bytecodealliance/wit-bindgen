# The `*.wit` format

This is intended to document the `*.wit` format as it exists today. The goal is
to provide an overview to understand what features `wit` files give you and how
they're structured. This isn't intended to be a formal grammar, although it's
expected that one day we'll have a formal grammar for `*.wit` files.

If you're curious to give things a spin try out the [online
demo](https://bytecodealliance.github.io/wit-bindgen/) of `wit-bindgen` where
you can input `*.wit` on the left and see output of generated bindings for
languages on the right. If you're looking to start you can try out the
"markdown" output mode which generates documentation for the input document on
the left.

## Lexical structure

The `wit` format is a curly-braced-based format where whitespace is optional (but
recommended). It is intended to be easily human readable and supports features
like comments, multi-line comments, and custom identifiers. A `wit` document
is parsed as a unicode string, and when stored in a file is expected to be
encoded as UTF-8.

Additionally, wit files must not contain any bidirectional override scalar values,
control codes other than newline, carriage return, and horizontal tab, or
codepoints that Unicode officially deprecates or strongly discourages.

The current structure of tokens are:

```wit
token ::= whitespace
        | comment
        | operator
        | keyword
        | identifier
```

Whitespace and comments are ignored when parsing structures defined elsewhere
here.

### Whitespace

A `whitespace` token in `*.wit` is a space, a newline, a carriage return, or a
tab character:

```wit
whitespace ::= ' ' | '\n' | '\r' | '\t'
```

### Comments

A `comment` token in `*.wit` is either a line comment preceded with `//` which
ends at the next newline (`\n`) character or it's a block comment which starts
with `/*` and ends with `*/`. Note that block comments are allowed to be nested
and their delimiters must be balanced

```wit
comment ::= '//' character-that-isnt-a-newline*
          | '/*' any-unicode-character* '*/'
```

There is a special type of comment called `documentation comment`. A
`doc-comment` is either a line comment preceded with `///` whichends at the next
newline (`\n`) character or it's a block comment which starts with `/**` and ends
with `*/`. Note that block comments are allowed to be nested and their delimiters
must be balanced

```wit
doc-comment ::= '///' character-that-isnt-a-newline*
          | '/**' any-unicode-character* '*/'
```

### Operators

There are some common operators in the lexical structure of `wit` used for
various constructs. Note that delimiters such as `{` and `(` must all be
balanced.

```wit
operator ::= '=' | ',' | ':' | ';' | '(' | ')' | '{' | '}' | '<' | '>' | '*' | '->'
```

### Keywords

Certain identifiers are reserved for use in `wit` documents and cannot be used
bare as an identifier. These are used to help parse the format, and the list of
keywords is still in flux at this time but the current set is:

```wit
keyword ::= 'use'
          | 'type'
          | 'resource'
          | 'func'
          | 'u8' | 'u16' | 'u32' | 'u64'
          | 's8' | 's16' | 's32' | 's64'
          | 'float32' | 'float64'
          | 'char'
          | 'handle'
          | 'record'
          | 'enum'
          | 'flags'
          | 'variant'
          | 'union'
          | 'bool'
          | 'string'
          | 'option'
          | 'list'
          | 'expected'
          | 'unit'
          | 'as'
          | 'from'
          | 'static'
          | 'interface'
          | 'tuple'
          | 'async'
          | 'future'
          | 'stream'
```

## Top-level items

A `wit` document is a sequence of items specified at the top level. These items
come one after another and it's recommended to separate them with newlines for
readability but this isn't required.

## Item: `use`

A `use` statement enables importing type or resource definitions from other
wit documents. The structure of a use statement is:

```wit
use * from other-file
use { a, list, of, names } from another-file
use { name as other-name } from yet-another-file
```

Specifically the structure of this is:

```wit
use-item ::= 'use' use-names 'from' id

use-names ::= '*'
            | '{' use-names-list '}'

use-names-list ::= use-names-item
                 | use-names-item ',' use-names-list?

use-names-item ::= id
                 | id 'as' id
```

Note: Here `use-names-list?` means at least one `use-name-list` term.

## Items: type

There are a number of methods of defining types in a `wit` document, and all of
the types that can be defined in `wit` are intended to map directly to types in
the [interface types specification](https://github.com/WebAssembly/interface-types).

### Item: `type` (alias)

A `type` statement declares a new named type in the `wit` document. This name can
be later referred to when defining items using this type. This construct is
similar to a type alias in other languages

```wit
type my-awesome-u32 = u32
type my-complicated-tuple = tuple<u32, s32, string>
```

Specifically the structure of this is:

```wit
type-item ::= 'type' id '=' ty
```

### Item: `record` (bag of named fields)

A `record` statement declares a new named structure with named fields. Records
are similar to a `struct` in many languages. Instances of a `record` always have
their fields defined.

```wit
record pair {
    x: u32,
    y: u32,
}

record person {
    name: string,
    age: u32,
    has-lego-action-figure: bool,
}
```

Specifically the structure of this is:

```wit
record-item ::= 'record' id '{' record-fields '}'

record-fields ::= record-field
                | record-field ',' record-fields?

record-field ::= id ':' ty
```

### Item: `flags` (bag-of-bools)

A `flags` statement defines a new `record`-like structure where all the fields
are booleans. The `flags` type is distinct from `record` in that it typically is
represented as a bit flags representation in the canonical ABI. For the purposes
of type-checking, however, it's simply syntactic sugar for a record-of-booleans.

```wit
flags properties {
    lego,
    marvel-superhero,
    supervillan,
}

// type-wise equivalent to:
//
// record properties {
//     lego: bool,
//     marvel-superhero: bool,
//     supervillan: bool,
// }
```

Specifically the structure of this is:

```wit
flags-items ::= 'flags' id '{' flags-fields '}'

flags-fields ::= id,
               | id ',' flags-fields?
```

### Item: `variant` (one of a set of types)

A `variant` statement defines a new type where instances of the type match
exactly one of the variants listed for the type. This is similar to a "sum" type
in algebraic datatypes (or an `enum` in Rust if you're familiar with it).
Variants can be thought of as tagged unions as well.

Each case of a variant can have an optional type associated with it which is
present when values have that particular case's tag.

All `variant` type must have at least one case specified.

```wit
variant filter {
    all,
    none,
    some(list<string>),
}
```

Specifically the structure of this is:

```wit
variant-items ::= 'variant' id '{' variant-cases '}'

variant-cases ::= variant-case,
                | variant-case ',' variant-cases?

variant-case ::= id
               | id '(' ty ')'
```

### Item: `enum` (variant but with no payload)

An `enum` statement defines a new type which is semantically equivalent to a
`variant` where none of the cases have a payload type. This is special-cased,
however, to possibly have a different representation in the language ABIs or
have different bindings generated in for languages.

```wit
enum color {
    red,
    green,
    blue,
    yellow,
    other,
}

// type-wise equivalent to:
//
// variant color {
//     red,
//     green,
//     blue,
//     yellow,
//     other,
// }
```

Specifically the structure of this is:

```wit
enum-items ::= 'enum' id '{' enum-cases '}'

enum-cases ::= id,
             | id ',' enum-cases?
```

### Item: `union` (variant but with no case names)

A `union` statement defines a new type which is semantically equivalent to a
`variant` where all of the cases have a payload type and the case names are
numerical. This is special-cased, however, to possibly have a different
representation in the language ABIs or have different bindings generated in for
languages.

```wit
union configuration {
    string,
    list<string>,
}

// type-wise equivalent to:
//
// variant configuration {
//     0(string),
//     1(list<string>),
// }
```

Specifically the structure of this is:

```wit
union-items ::= 'union' id '{' union-cases '}'

union-cases ::= ty,
              | ty ',' union-cases?
```

## Item: `func`

Functions can also be defined in a `*.wit` document. Functions have a name,
parameters, and results. Functions can optionally also be declared as `async`
functions.

```wit
thunk: func()
fibonacci: func(n: u32) -> u32
sleep: async func(ms: u64)
```

Specifically functions have the structure:

```wit
func-item ::= id ':' 'async'? 'func' '(' func-args ')' func-ret

func-args ::= func-arg
            | func-arg ',' func-args?

func-arg ::= id ':' ty

func-ret ::= nil
           | '->' ty
```

## Item: `resource`

Resources represent a value that has a hidden representation not known to the
outside world. This means that the resource is operated on through a "handle" (a
pointer of sorts). Resources also have ownership associated with them and
languages will have to manage the lifetime of resources manually (they're
similar to file descriptors).

Resources can also optionally have functions defined within them which adds an
implicit "self" argument as the first argument to each function of the same type
of the including resource, unless the function is flagged as `static`.

```wit
resource file-descriptor

resource request {
    static new: func() -> request

    body: async func() -> list<u8>
    headers: func() -> list<string>
}
```

Specifically resources have the structure:

```wit
resource-item ::= 'resource' id resource-contents

resource-contents ::= nil
                    | '{' resource-defs '}'

resource-defs ::= resource-def resource-defs?

resource-def ::= 'static'? func-item
```

## Types

As mentioned previously the intention of `wit` is to allow defining types
corresponding to the interface types specification. Many of the top-level items
above are introducing new named types but "anonymous" types are also supported,
such as built-ins. For example:

```wit
type number = u32
type fallible-function-result = expected<u32, string>
type headers = list<string>
```

Specifically the following types are available:

```wit
ty ::= 'u8' | 'u16' | 'u32' | 'u64'
     | 's8' | 's16' | 's32' | 's64'
     | 'float32' | 'float64'
     | 'char'
     | 'bool'
     | 'string'
     | 'unit'
     | tuple
     | list
     | option
     | expected
     | future
     | stream
     | id

tuple ::= 'tuple' '<' tuple-list '>'
tuple-list ::= ty
             | ty ',' tuple-list?

list ::= 'list' '<' ty '>'

option ::= 'option' '<' ty '>'

expected ::= 'expected' '<' ty ',' ty '>'

future ::= 'future' '<' ty '>'

stream ::= 'stream' '<' ty ',' ty '>'
```

The `tuple` type is semantically equivalent to a `record` with numerical fields,
but it frequently can have language-specific meaning so it's provided as a
first-class type.

Similarly the `option` and `expected` types are semantically equivalent to the
variants:

```wit
variant option {
    none,
    some(ty),
}

variant expected {
    ok(ok-ty)
    err(err-ty),
}
```

These types are so frequently used and frequently have language-specific
meanings though so they're also provided as first-class types.

Finally the last case of a `ty` is simply an `id` which is intended to refer to
another type or resource defined in the document. Note that definitions can come
through a `use` statement or they can be defined locally.

## Identifiers

Identifiers in `wit` can be defined with two different forms. The first is a
lower-case [stream-safe] [NFC] [kebab-case] identifier where each part delimited
by '-'s starts with a `XID_Start` scalar value with a zero Canonical Combining
Class:

```wit
foo: func(bar: u32)

red-green-blue: func(r: u32, g: u32, b: u32)
```

This form can't name identifiers which have the same name as wit keywords, so
the second form is the same syntax with the same restrictions as the first, but
prefixed with '%':

```wit
%foo: func(%bar: u32)

%red-green-blue: func(%r: u32, %g: u32, %b: u32)

// This form also supports identifiers that would otherwise be keywords.
%variant: func(%enum: s32)
```

[kebab-case]: https://en.wikipedia.org/wiki/Letter_case#Kebab_case
[Unicode identifier]: http://www.unicode.org/reports/tr31/
[stream-safe]: https://unicode.org/reports/tr15/#Stream_Safe_Text_Format
[NFC]: https://unicode.org/reports/tr15/#Norm_Forms

## Name resolution

A `wit` document is resolved after parsing to ensure that all names resolve
correctly. For example this is not a valid `wit` document:

```wit
type foo = bar  // ERROR: name `bar` not defined
```

Type references primarily happen through the `id` production of `ty`.

Additionally names in a `wit` document can only be defined once:

```wit
type foo = u32
type foo = u64  // ERROR: name `foo` already defined
```

Names do not need to be defined before they're used (unlike in C or C++),
it's ok to define a type after it's used:

```wit
type foo = bar

record bar {
    age: u32,
}
```

Types, however, cannot be recursive:

```wit
type foo = foo  // ERROR: cannot refer to itself

record bar1 {
    a: bar2,
}

record bar2 {
    a: bar1,  // ERROR: record cannot refer to itself
}
```

The intention of `wit` is that it maps down to interface types, so the goal of
name resolution is to effectively create the type section of a wasm module using
interface types. The restrictions about self-referential types and such come
from how types can be defined in the interface types section. Additionally
definitions of named types such as `record foo { ... }` are intended to map
roughly to declarations in the type section of new types.
