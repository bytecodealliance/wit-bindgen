# The `*.world` format

This is intended to document the `*.world` format as it currently exists. The goal is to provide an overview to understand what features `world` files give you and how they're structured. This isn't intended to be a formal grammar, although it's expected that one day we'll have a formal grammar for `*.world` files.

Conceptually `world` documents act as a contract between runtimes and components. I.e., hosts implement worlds and components target them.

## Lexical structure

> TODO: Mention lexical additions relative to WIT.md

## Top-level items

A `world` document is a sequence of items specified at the top level. These items come one after another and it's recommended to separate them with newlines for readability but  this isn't required.

Conceretely, the structure of a `world` file is:

```
top-level ::= (import | export | extend | use-item | type-item)*
```

### Item: `import`

An `import` statement imports an instance, function, or value.

The structure of an import statment is:

```
import ::= 'import' id ':' type-use
```

Example:

```world
import backends: { *: "wasi:http/Handler" }
import console: "wasi:logging/Logger"
```

### Item: `export`

An `export` statement exports an instance, function, or value.

The structure of an export statment is:

```
export ::= 'export' id ':' type-use
```

Example:

```world
export backends: { +: "wasi:http/Handler" }
export handler: "wasi:http/Handler"
```

### Item: `extends`

An `extends` statement defines a subtype relationship with the referenced profile as the super type.

The structure of an extend statement is:

```
extends ::= 'extends' pathlit
```

Example:

```world
// Service.world -- Common service profile

import console: "wasi:logging/Logger"
import logs: { *: "wasi:logging/Logger" }
import config: { *: "wasi:config/Value" }
import secrets: { *: "wasi:config/Secret" }
import metrics: { *: "wasi:metrics/Counter" }

```

```world
// http/Service.world -- An HTTP service profile

extends "wasi:Service"
export "wasi:http/Handler"
```

### Item: `use-item`

> TODO: Does `use` apply to `wit` files?

### Item: `type-item`

A `type-item` statement declares a new named type in the `world` document. This name can be later referred to when defining `import` and `export` items. 

The structure of a type-item statement is:

```
type-item ::= 'type' id '=' extern-type
```

Example:

```world
type message = record { x: string }
type greeter = func(msg: message) -> expected<unit, string>
```

> TODO: Discuss semantics of type item

# Grammar
```
    top-level ::= (extend | import | export  | use-item | type-item)*
      extends ::= 'extends' worldfile
       import ::= 'import' id ':' type-use
       export ::= 'export' id ':' type-use
    type-item ::= 'type' id '=' extern-type
     type-use ::= id | extern-type
    func-type ::= 'async'? 'func' '(' func-args? ')' func-ret?
    func-args ::= func-arg | func-arg ',' func-args?
     func-arg ::= id ':' val-type
     func-ret ::= '->' val-type
     val-type ::= ty | ... | 'any'
  extern-type ::= instance-type | func-type | val-type
instance-type ::= '{' export* '}' | '{' ('*' | '+') extern-type '}' | witfile
```

> NOTE: `use-def` as defined in [WIT.md](https://github.com/bytecodealliance/wit-bindgen/blob/main/WIT.md#item-use)

> NOTE: `ty` as defined in [WIT.md](https://github.com/bytecodealliance/wit-bindgen/blob/main/WIT.md#item-ty)

> TODO: Describe `*` & `+` instance types.