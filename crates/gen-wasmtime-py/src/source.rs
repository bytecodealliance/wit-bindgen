use heck::*;
use wit_bindgen_gen_core::wit_parser::*;

use crate::dependencies::Dependencies;

/// A [Source] represents some unit of Python code
/// and keeps track of its indent.
#[derive(Default)]
pub struct Source {
    s: String,
    indent: usize,
}

impl Source {
    /// Appends a string slice to this [Source].
    ///
    /// Strings without newlines, they are simply appended.
    /// Strings with newlines are appended and also new lines
    /// are indented based on the current indent level.
    pub fn push_str(&mut self, src: &str) {
        let lines = src.lines().collect::<Vec<_>>();
        let mut trim = None;
        for (i, line) in lines.iter().enumerate() {
            self.s.push_str(if lines.len() == 1 {
                line
            } else {
                let trim = match trim {
                    Some(n) => n,
                    None => {
                        let val = line.len() - line.trim_start().len();
                        if !line.is_empty() {
                            trim = Some(val);
                        }
                        val
                    }
                };
                line.get(trim..).unwrap_or("")
            });
            if i != lines.len() - 1 || src.ends_with("\n") {
                self.newline();
            }
        }
    }

    /// Prints the documentation as comments
    /// e.g.
    /// > \# Line one of docs node
    /// >
    /// > \# Line two of docs node
    pub fn comment(&mut self, docs: &Docs) {
        let docs = match &docs.contents {
            Some(docs) => docs,
            None => return,
        };
        for line in docs.lines() {
            self.push_str(&format!("# {}\n", line));
        }
    }

    /// Prints the documentation as comments
    /// e.g.
    /// > """
    /// >
    /// > Line one of docs node
    /// >
    /// > Line two of docs node
    /// >
    /// > """
    pub fn docstring(&mut self, docs: &Docs) {
        let docs = match &docs.contents {
            Some(docs) => docs,
            None => return,
        };
        let triple_quote = r#"""""#;
        self.push_str(triple_quote);
        self.newline();
        for line in docs.lines() {
            self.push_str(line);
            self.newline();
        }
        self.push_str(triple_quote);
        self.newline();
    }

    /// Indent the source one level.
    pub fn indent(&mut self) {
        self.indent += 4;
        self.s.push_str("    ");
    }

    /// Unindent, or in Python terms "dedent",
    /// the source one level.
    pub fn dedent(&mut self) {
        self.indent -= 4;
        assert!(self.s.ends_with("    "));
        self.s.pop();
        self.s.pop();
        self.s.pop();
        self.s.pop();
    }

    /// Go to the next line and apply any indent.
    pub fn newline(&mut self) {
        self.s.push_str("\n");
        for _ in 0..self.indent {
            self.s.push_str(" ");
        }
    }
}

impl std::ops::Deref for Source {
    type Target = str;
    fn deref(&self) -> &str {
        &self.s
    }
}

impl From<Source> for String {
    fn from(s: Source) -> String {
        s.s
    }
}

/// [SourceBuilder] combines together a [Source]
/// with other contextual information and state.
///
/// This allows you to generate code for the Source using
/// high-level tools that take care of updating dependencies
/// and retrieving interface details.
///
/// You can create a [SourceBuilder] easily using a [Source]
/// ```
/// # use wit_bindgen_gen_wasmtime_py::dependencies::Dependencies;
/// # use wit_bindgen_gen_core::wit_parser::{Interface, Type};
/// # use wit_bindgen_gen_wasmtime_py::source::Source;
/// # let mut deps = Dependencies::default();
/// # let mut interface = Interface::default();
/// # let iface = &interface;
/// let mut source = Source::default();
/// let mut builder = source.builder(&mut deps, iface);
/// builder.print_ty(&Type::Bool, false);
/// ```
pub struct SourceBuilder<'s, 'd, 'i> {
    source: &'s mut Source,
    pub deps: &'d mut Dependencies,
    iface: &'i Interface,
}

impl<'s, 'd, 'i> Source {
    /// Create a [SourceBuilder] for the current source.
    pub fn builder(
        &'s mut self,
        deps: &'d mut Dependencies,
        iface: &'i Interface,
    ) -> SourceBuilder<'s, 'd, 'i> {
        SourceBuilder {
            source: self,
            deps,
            iface,
        }
    }
}

impl<'s, 'd, 'i> SourceBuilder<'s, 'd, 'i> {
    /// See [Dependencies::pyimport].
    pub fn pyimport<'a>(&mut self, module: &str, name: impl Into<Option<&'a str>>) {
        self.deps.pyimport(module, name)
    }

    /// Appends a type's Python representation to this `Source`.
    /// Records any required intrinsics and imports in the `deps`.
    /// Uses Python forward reference syntax (e.g. 'Foo')
    /// on the root type only if `forward_ref` is true.
    pub fn print_ty(&mut self, ty: &Type, forward_ref: bool) {
        match ty {
            Type::Unit => self.push_str("None"),
            Type::Bool => self.push_str("bool"),
            Type::U8
            | Type::S8
            | Type::U16
            | Type::S16
            | Type::U32
            | Type::S32
            | Type::U64
            | Type::S64 => self.push_str("int"),
            Type::Float32 | Type::Float64 => self.push_str("float"),
            Type::Char => self.push_str("str"),
            Type::String => self.push_str("str"),
            Type::Handle(id) => {
                if forward_ref {
                    self.push_str("'");
                }
                let handle_name = &self.iface.resources[*id].name.to_camel_case();
                self.source.push_str(handle_name);
                if forward_ref {
                    self.push_str("'");
                }
            }
            Type::Id(id) => {
                let ty = &self.iface.types[*id];
                if let Some(name) = &ty.name {
                    self.push_str(&name.to_camel_case());
                    return;
                }
                match &ty.kind {
                    TypeDefKind::Type(t) => self.print_ty(t, forward_ref),
                    TypeDefKind::Tuple(t) => self.print_tuple(t),
                    TypeDefKind::Record(_)
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Enum(_)
                    | TypeDefKind::Variant(_)
                    | TypeDefKind::Union(_) => {
                        unreachable!()
                    }
                    TypeDefKind::Option(t) => {
                        self.deps.pyimport("typing", "Optional");
                        self.push_str("Optional[");
                        self.print_ty(t, true);
                        self.push_str("]");
                    }
                    TypeDefKind::Expected(e) => {
                        self.deps.needs_expected = true;
                        self.push_str("Expected[");
                        self.print_ty(&e.ok, true);
                        self.push_str(", ");
                        self.print_ty(&e.err, true);
                        self.push_str("]");
                    }
                    TypeDefKind::List(t) => self.print_list(t),
                    TypeDefKind::Future(t) => {
                        self.push_str("Future[");
                        self.print_ty(t, true);
                        self.push_str("]");
                    }
                    TypeDefKind::Stream(s) => {
                        self.push_str("Stream[");
                        self.print_ty(&s.element, true);
                        self.push_str(", ");
                        self.print_ty(&s.end, true);
                        self.push_str("]");
                    }
                }
            }
        }
    }

    /// Appends a tuple type's Python representation to this `Source`.
    /// Records any required intrinsics and imports in the `deps`.
    /// Uses Python forward reference syntax (e.g. 'Foo') for named type parameters.
    pub fn print_tuple(&mut self, tuple: &Tuple) {
        if tuple.types.is_empty() {
            return self.push_str("None");
        }
        self.deps.pyimport("typing", "Tuple");
        self.push_str("Tuple[");
        for (i, t) in tuple.types.iter().enumerate() {
            if i > 0 {
                self.push_str(", ");
            }
            self.print_ty(t, true);
        }
        self.push_str("]");
    }

    /// Appends a Python type representing a sequence of the `element` type to this `Source`.
    /// If the element type is `Type::U8`, the result type is `bytes` otherwise it is a `List[T]`
    /// Records any required intrinsics and imports in the `deps`.
    /// Uses Python forward reference syntax (e.g. 'Foo') for named type parameters.
    pub fn print_list(&mut self, element: &Type) {
        match element {
            Type::U8 => self.push_str("bytes"),
            t => {
                self.deps.pyimport("typing", "List");
                self.push_str("List[");
                self.print_ty(t, true);
                self.push_str("]");
            }
        }
    }

    /// Print variable declaration.
    /// Brings name into scope and binds type to it.
    pub fn print_var_declaration<'a>(&mut self, name: &'a str, ty: &Type) {
        self.push_str(name);
        self.push_str(": ");
        self.print_ty(ty, true);
        self.push_str("\n");
    }

    pub fn print_sig(&mut self, func: &Function, in_import: bool) -> Vec<String> {
        if !in_import {
            if let FunctionKind::Static { .. } = func.kind {
                self.push_str("@classmethod\n");
            }
        }
        self.source.push_str("def ");
        match &func.kind {
            FunctionKind::Method { .. } => self.source.push_str(&func.item_name().to_snake_case()),
            FunctionKind::Static { .. } if !in_import => {
                self.source.push_str(&func.item_name().to_snake_case())
            }
            _ => self.source.push_str(&func.name.to_snake_case()),
        }
        if in_import {
            self.source.push_str("(self");
        } else if let FunctionKind::Static { .. } = func.kind {
            self.source.push_str("(cls, caller: wasmtime.Store, obj: '");
            self.source.push_str(&self.iface.name.to_camel_case());
            self.source.push_str("'");
        } else {
            self.source.push_str("(self, caller: wasmtime.Store");
        }
        let mut params = Vec::new();
        for (i, (param, ty)) in func.params.iter().enumerate() {
            if i == 0 {
                if let FunctionKind::Method { .. } = func.kind {
                    params.push("self".to_string());
                    continue;
                }
            }
            self.source.push_str(", ");
            self.source.push_str(&param.to_snake_case());
            params.push(param.to_snake_case());
            self.source.push_str(": ");
            self.print_ty(ty, true);
        }
        self.source.push_str(") -> ");
        self.print_ty(&func.result, true);
        params
    }

    /// Print a wrapped union definition.
    /// e.g.
    /// ```py
    /// @dataclass
    /// class Foo0:
    ///     value: int
    ///  
    /// @dataclass
    /// class Foo1:
    ///     value: int
    ///  
    /// Foo = Union[Foo0, Foo1]
    /// ```
    pub fn print_union_wrapped(&mut self, name: &str, union: &Union, docs: &Docs) {
        self.deps.pyimport("dataclasses", "dataclass");
        let mut cases = Vec::new();
        let name = name.to_camel_case();
        for (i, case) in union.cases.iter().enumerate() {
            self.source.push_str("@dataclass\n");
            let name = format!("{name}{i}");
            self.source.push_str(&format!("class {name}:\n"));
            self.source.indent();
            self.source.docstring(&case.docs);
            self.source.push_str("value: ");
            self.print_ty(&case.ty, true);
            self.source.newline();
            self.source.dedent();
            self.source.newline();
            cases.push(name);
        }

        self.deps.pyimport("typing", "Union");
        self.source.comment(docs);
        self.source
            .push_str(&format!("{name} = Union[{}]\n", cases.join(", ")));
        self.source.newline();
    }

    pub fn print_union_raw(&mut self, name: &str, union: &Union, docs: &Docs) {
        self.deps.pyimport("typing", "Union");
        self.source.comment(docs);
        for case in union.cases.iter() {
            self.source.comment(&case.docs);
        }
        self.source.push_str(&name.to_camel_case());
        self.source.push_str(" = Union[");
        let mut first = true;
        for case in union.cases.iter() {
            if !first {
                self.source.push_str(",");
            }
            self.print_ty(&case.ty, true);
            first = false;
        }
        self.source.push_str("]\n\n");
    }
}

impl<'s, 'd, 'i> std::ops::Deref for SourceBuilder<'s, 'd, 'i> {
    type Target = Source;
    fn deref(&self) -> &Source {
        &self.source
    }
}

impl<'s, 'd, 'i> std::ops::DerefMut for SourceBuilder<'s, 'd, 'i> {
    fn deref_mut(&mut self) -> &mut Source {
        &mut self.source
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    #[test]
    fn simple_append() {
        let mut s = Source::default();
        s.push_str("x");
        assert_eq!(s.s, "x");
        s.push_str("y");
        assert_eq!(s.s, "xy");
        s.push_str("z ");
        assert_eq!(s.s, "xyz ");
        s.push_str(" a ");
        assert_eq!(s.s, "xyz  a ");
        s.push_str("\na");
        assert_eq!(s.s, "xyz  a \na");
    }

    #[test]
    fn trim_ws() {
        let mut s = Source::default();
        s.push_str("def foo():\n  return 1\n");
        assert_eq!(s.s, "def foo():\n  return 1\n");
    }

    #[test]
    fn print_ty_forward_ref() {
        let mut deps = Dependencies::default();
        let mut iface = Interface::default();
        // Set up a Resource type to refer to
        let resource_id = iface.resources.alloc(Resource {
            docs: Docs::default(),
            name: "foo".into(),
            supertype: None,
            foreign_module: None,
        });
        iface.resource_lookup.insert("foo".into(), resource_id);
        let handle_ty = Type::Handle(resource_id);
        // ForwardRef usage can be controlled by an argument to print_ty
        let mut s1 = Source::default();
        let mut builder = s1.builder(&mut deps, &iface);
        builder.print_ty(&handle_ty, true);
        drop(builder);
        assert_eq!(s1.s, "'Foo'");

        let mut s2 = Source::default();
        let mut builder = s2.builder(&mut deps, &iface);
        builder.print_ty(&handle_ty, false);
        drop(builder);
        assert_eq!(s2.s, "Foo");

        // ForwardRef is used for any types within other types
        // Even if the outer type is itself not allowed to be one
        let option_id = iface.types.alloc(TypeDef {
            docs: Docs::default(),
            kind: TypeDefKind::Option(handle_ty),
            name: None,
            foreign_module: None,
        });
        let option_ty = Type::Id(option_id);
        let mut s3 = Source::default();
        let mut builder = s3.builder(&mut deps, &iface);
        builder.print_ty(&option_ty, false);
        drop(builder);
        assert_eq!(s3.s, "Optional['Foo']");
    }

    #[test]
    fn print_list_bytes() {
        // If the element type is u8, it is interpreted as `bytes`
        let mut deps = Dependencies::default();
        let iface = Interface::default();
        let mut source = Source::default();
        let mut builder = source.builder(&mut deps, &iface);
        builder.print_list(&Type::U8);
        drop(builder);
        assert_eq!(source.s, "bytes");
        assert_eq!(deps.pyimports, BTreeMap::default());
    }

    #[test]
    fn print_list_non_bytes() {
        // If the element type is u8, it is interpreted as `bytes`
        let mut deps = Dependencies::default();
        let iface = Interface::default();
        let mut source = Source::default();
        let mut builder = source.builder(&mut deps, &iface);
        builder.print_list(&Type::Float32);
        drop(builder);
        assert_eq!(source.s, "List[float]");
        assert_eq!(
            deps.pyimports,
            BTreeMap::from([("typing".into(), Some(BTreeSet::from(["List".into()])))])
        );
    }
}
