use heck::*;
use pulldown_cmark::{html, Event, LinkType, Parser, Tag};
use std::collections::HashMap;
use wit_bindgen_core::{wit_parser, Direction, Files, Generator, Source};
use wit_parser::*;

#[derive(Default)]
pub struct Markdown {
    src: Source,
    opts: Opts,
    sizes: SizeAlign,
    hrefs: HashMap<String, String>,
    funcs: usize,
    types: usize,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "structopt", derive(structopt::StructOpt))]
pub struct Opts {
    // ...
}

impl Opts {
    pub fn build(&self) -> Markdown {
        let mut r = Markdown::new();
        r.opts = self.clone();
        r
    }
}

impl Markdown {
    pub fn new() -> Markdown {
        Markdown::default()
    }

    fn print_ty(&mut self, iface: &Interface, ty: &Type, skip_name: bool) {
        match ty {
            Type::Unit => self.src.push_str("`unit`"),
            Type::Bool => self.src.push_str("`bool`"),
            Type::U8 => self.src.push_str("`u8`"),
            Type::S8 => self.src.push_str("`s8`"),
            Type::U16 => self.src.push_str("`u16`"),
            Type::S16 => self.src.push_str("`s16`"),
            Type::U32 => self.src.push_str("`u32`"),
            Type::S32 => self.src.push_str("`s32`"),
            Type::U64 => self.src.push_str("`u64`"),
            Type::S64 => self.src.push_str("`s64`"),
            Type::Float32 => self.src.push_str("`float32`"),
            Type::Float64 => self.src.push_str("`float64`"),
            Type::Char => self.src.push_str("`char`"),
            Type::String => self.src.push_str("`string`"),
            Type::Handle(id) => {
                self.src.push_str("handle<");
                self.src.push_str(&iface.resources[*id].name);
                self.src.push_str(">");
            }
            Type::Id(id) => {
                let ty = &iface.types[*id];
                if !skip_name {
                    if let Some(name) = &ty.name {
                        self.src.push_str("[`");
                        self.src.push_str(name);
                        self.src.push_str("`](#");
                        self.src.push_str(&name.to_snake_case());
                        self.src.push_str(")");
                        return;
                    }
                }
                match &ty.kind {
                    TypeDefKind::Type(t) => self.print_ty(iface, t, false),
                    TypeDefKind::Tuple(t) => {
                        self.src.push_str("(");
                        for (i, t) in t.types.iter().enumerate() {
                            if i > 0 {
                                self.src.push_str(", ");
                            }
                            self.print_ty(iface, t, false);
                        }
                        self.src.push_str(")");
                    }
                    TypeDefKind::Record(_)
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Enum(_)
                    | TypeDefKind::Variant(_)
                    | TypeDefKind::Union(_) => {
                        unreachable!()
                    }
                    TypeDefKind::Option(t) => {
                        self.src.push_str("option<");
                        self.print_ty(iface, t, false);
                        self.src.push_str(">");
                    }
                    TypeDefKind::Expected(e) => {
                        self.src.push_str("expected<");
                        self.print_ty(iface, &e.ok, false);
                        self.src.push_str(", ");
                        self.print_ty(iface, &e.err, false);
                        self.src.push_str(">");
                    }
                    TypeDefKind::List(t) => {
                        self.src.push_str("list<");
                        self.print_ty(iface, t, false);
                        self.src.push_str(">");
                    }
                    TypeDefKind::Future(t) => {
                        self.src.push_str("future<");
                        self.print_ty(iface, t, false);
                        self.src.push_str(">");
                    }
                    TypeDefKind::Stream(s) => {
                        self.src.push_str("stream<");
                        self.print_ty(iface, &s.element, false);
                        self.src.push_str(", ");
                        self.print_ty(iface, &s.end, false);
                        self.src.push_str(">");
                    }
                }
            }
        }
    }

    fn docs(&mut self, docs: &Docs) {
        let docs = match &docs.contents {
            Some(docs) => docs,
            None => return,
        };
        for line in docs.lines() {
            self.src.push_str(line.trim());
            self.src.push_str("\n");
        }
    }

    fn print_type_header(&mut self, name: &str) {
        if self.types == 0 {
            self.src.push_str("# Types\n\n");
        }
        self.types += 1;
        self.src.push_str(&format!(
            "## <a href=\"#{}\" name=\"{0}\"></a> `{}`: ",
            name.to_snake_case(),
            name,
        ));
        self.hrefs
            .insert(name.to_string(), format!("#{}", name.to_snake_case()));
    }

    fn print_type_info(&mut self, ty: TypeId, docs: &Docs) {
        self.docs(docs);
        self.src.push_str("\n");
        self.src
            .push_str(&format!("Size: {}, ", self.sizes.size(&Type::Id(ty))));
        self.src
            .push_str(&format!("Alignment: {}\n", self.sizes.align(&Type::Id(ty))));
    }
}

impl Generator for Markdown {
    fn preprocess_one(&mut self, iface: &Interface, _dir: Direction) {
        self.sizes.fill(iface);
    }

    fn type_record(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        record: &Record,
        docs: &Docs,
    ) {
        self.print_type_header(name);
        self.src.push_str("record\n\n");
        self.print_type_info(id, docs);
        self.src.push_str("\n### Record Fields\n\n");
        for field in record.fields.iter() {
            self.src.push_str(&format!(
                "- <a href=\"{r}.{f}\" name=\"{r}.{f}\"></a> [`{name}`](#{r}.{f}): ",
                r = name.to_snake_case(),
                f = field.name.to_snake_case(),
                name = field.name,
            ));
            self.hrefs.insert(
                format!("{}::{}", name, field.name),
                format!("#{}.{}", name.to_snake_case(), field.name.to_snake_case()),
            );
            self.print_ty(iface, &field.ty, false);
            self.src.indent(1);
            self.src.push_str("\n\n");
            self.docs(&field.docs);
            self.src.deindent(1);
            self.src.push_str("\n");
        }
    }

    fn type_tuple(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        tuple: &Tuple,
        docs: &Docs,
    ) {
        self.print_type_header(name);
        self.src.push_str("tuple\n\n");
        self.print_type_info(id, docs);
        self.src.push_str("\n### Tuple Fields\n\n");
        for (i, ty) in tuple.types.iter().enumerate() {
            self.src.push_str(&format!(
                "- <a href=\"{r}.{f}\" name=\"{r}.{f}\"></a> [`{name}`](#{r}.{f}): ",
                r = name.to_snake_case(),
                f = i,
                name = i,
            ));
            self.hrefs.insert(
                format!("{}::{}", name, i),
                format!("#{}.{}", name.to_snake_case(), i),
            );
            self.print_ty(iface, ty, false);
            self.src.push_str("\n");
        }
    }

    fn type_flags(
        &mut self,
        _iface: &Interface,
        id: TypeId,
        name: &str,
        flags: &Flags,
        docs: &Docs,
    ) {
        self.print_type_header(name);
        self.src.push_str("record\n\n");
        self.print_type_info(id, docs);
        self.src.push_str("\n### Record Fields\n\n");
        for (i, flag) in flags.flags.iter().enumerate() {
            self.src.push_str(&format!(
                "- <a href=\"{r}.{f}\" name=\"{r}.{f}\"></a> [`{name}`](#{r}.{f}): ",
                r = name.to_snake_case(),
                f = flag.name.to_snake_case(),
                name = flag.name,
            ));
            self.hrefs.insert(
                format!("{}::{}", name, flag.name),
                format!("#{}.{}", name.to_snake_case(), flag.name.to_snake_case()),
            );
            self.src.indent(1);
            self.src.push_str("\n\n");
            self.docs(&flag.docs);
            self.src.deindent(1);
            self.src.push_str(&format!("Bit: {}\n", i));
            self.src.push_str("\n");
        }
    }

    fn type_variant(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        variant: &Variant,
        docs: &Docs,
    ) {
        self.print_type_header(name);
        self.src.push_str("variant\n\n");
        self.print_type_info(id, docs);
        self.src.push_str("\n### Variant Cases\n\n");
        for case in variant.cases.iter() {
            self.src.push_str(&format!(
                "- <a href=\"{v}.{c}\" name=\"{v}.{c}\"></a> [`{name}`](#{v}.{c})",
                v = name.to_snake_case(),
                c = case.name.to_snake_case(),
                name = case.name,
            ));
            self.hrefs.insert(
                format!("{}::{}", name, case.name),
                format!("#{}.{}", name.to_snake_case(), case.name.to_snake_case()),
            );
            self.src.push_str(": ");
            self.print_ty(iface, &case.ty, false);
            self.src.indent(1);
            self.src.push_str("\n\n");
            self.docs(&case.docs);
            self.src.deindent(1);
            self.src.push_str("\n");
        }
    }

    fn type_union(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        union: &Union,
        docs: &Docs,
    ) {
        self.print_type_header(name);
        self.src.push_str("union\n\n");
        self.print_type_info(id, docs);
        self.src.push_str("\n### Union Cases\n\n");
        let snake = name.to_snake_case();
        for (i, case) in union.cases.iter().enumerate() {
            self.src.push_str(&format!(
                "- <a href=\"{snake}.{i}\" name=\"{snake}.{i}\"></a> [`{i}`](#{snake}.{i})",
            ));
            self.hrefs
                .insert(format!("{name}::{i}"), format!("#{snake}.{i}"));
            self.src.push_str(": ");
            self.print_ty(iface, &case.ty, false);
            self.src.indent(1);
            self.src.push_str("\n\n");
            self.docs(&case.docs);
            self.src.deindent(1);
            self.src.push_str("\n");
        }
    }

    fn type_enum(&mut self, _iface: &Interface, id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        self.print_type_header(name);
        self.src.push_str("enum\n\n");
        self.print_type_info(id, docs);
        self.src.push_str("\n### Enum Cases\n\n");
        for case in enum_.cases.iter() {
            self.src.push_str(&format!(
                "- <a href=\"{v}.{c}\" name=\"{v}.{c}\"></a> [`{name}`](#{v}.{c})",
                v = name.to_snake_case(),
                c = case.name.to_snake_case(),
                name = case.name,
            ));
            self.hrefs.insert(
                format!("{}::{}", name, case.name),
                format!("#{}.{}", name.to_snake_case(), case.name.to_snake_case()),
            );
            self.src.indent(1);
            self.src.push_str("\n\n");
            self.docs(&case.docs);
            self.src.deindent(1);
            self.src.push_str("\n");
        }
    }

    fn type_option(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        payload: &Type,
        docs: &Docs,
    ) {
        self.print_type_header(name);
        self.src.push_str("option<");
        self.print_ty(iface, payload, false);
        self.src.push_str(">");
        self.print_type_info(id, docs);
    }

    fn type_expected(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        expected: &Expected,
        docs: &Docs,
    ) {
        self.print_type_header(name);
        self.src.push_str("expected<");
        self.print_ty(iface, &expected.ok, false);
        self.src.push_str(", ");
        self.print_ty(iface, &expected.err, false);
        self.src.push_str(">");
        self.print_type_info(id, docs);
    }

    fn type_resource(&mut self, iface: &Interface, ty: ResourceId) {
        drop((iface, ty));
    }

    fn type_alias(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.print_type_header(name);
        self.print_ty(iface, ty, true);
        self.src.push_str("\n\n");
        self.print_type_info(id, docs);
        self.src.push_str("\n");
    }

    fn type_list(&mut self, iface: &Interface, id: TypeId, name: &str, _ty: &Type, docs: &Docs) {
        self.type_alias(iface, id, name, &Type::Id(id), docs);
    }

    fn type_builtin(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.type_alias(iface, id, name, ty, docs)
    }

    fn import(&mut self, iface: &Interface, func: &Function) {
        if self.funcs == 0 {
            self.src.push_str("# Functions\n\n");
        }
        self.funcs += 1;

        self.src.push_str("----\n\n");
        self.src.push_str(&format!(
            "#### <a href=\"#{0}\" name=\"{0}\"></a> `",
            func.name.to_snake_case()
        ));
        self.hrefs
            .insert(func.name.clone(), format!("#{}", func.name.to_snake_case()));
        self.src.push_str(&func.name);
        self.src.push_str("` ");
        self.src.push_str("\n\n");
        self.docs(&func.docs);

        if func.params.len() > 0 {
            self.src.push_str("##### Params\n\n");
            for (name, ty) in func.params.iter() {
                self.src.push_str(&format!(
                    "- <a href=\"#{f}.{p}\" name=\"{f}.{p}\"></a> `{}`: ",
                    name,
                    f = func.name.to_snake_case(),
                    p = name.to_snake_case(),
                ));
                self.print_ty(iface, ty, false);
                self.src.push_str("\n");
            }
        }
        match &func.result {
            Type::Unit => {}
            ty => {
                self.src.push_str("##### Results\n\n");
                self.src.push_str(&format!(
                    "- <a href=\"#{f}.{p}\" name=\"{f}.{p}\"></a> `{}`: ",
                    "result",
                    f = func.name.to_snake_case(),
                    p = "result",
                ));
                self.print_ty(iface, ty, false);
                self.src.push_str("\n");
            }
        }

        self.src.push_str("\n");
    }

    fn export(&mut self, iface: &Interface, func: &Function) {
        self.import(iface, func);
    }

    fn finish_one(&mut self, _iface: &Interface, files: &mut Files) {
        let parser = Parser::new(&self.src);
        let mut events = Vec::new();
        for event in parser {
            if let Event::Code(code) = &event {
                if let Some(dst) = self.hrefs.get(code.as_ref()) {
                    let tag = Tag::Link(LinkType::Inline, dst.as_str().into(), "".into());
                    events.push(Event::Start(tag.clone()));
                    events.push(event.clone());
                    events.push(Event::End(tag));
                    continue;
                }
            }
            events.push(event);
        }
        let mut html_output = String::new();
        html::push_html(&mut html_output, events.into_iter());

        files.push("bindings.md", self.src.as_bytes());
        files.push("bindings.html", html_output.as_bytes());
    }
}
