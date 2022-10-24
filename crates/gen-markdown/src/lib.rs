use heck::*;
use pulldown_cmark::{html, Event, LinkType, Parser, Tag};
use std::collections::HashMap;
use std::fmt::Write;
use wit_bindgen_core::{
    uwriteln, wit_parser, Files, InterfaceGenerator as _, Source, WorldGenerator,
};
use wit_component::ComponentInterfaces;
use wit_parser::*;

#[derive(Default)]
struct Markdown {
    src: Source,
    opts: Opts,
    hrefs: HashMap<String, String>,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    // ...
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        let mut r = Markdown::default();
        r.opts = self.clone();
        Box::new(r)
    }
}

impl WorldGenerator for Markdown {
    fn import(&mut self, name: &str, iface: &Interface, _files: &mut Files) {
        uwriteln!(self.src, "# Import interface `{name}`\n");
        let mut gen = self.interface(iface);
        gen.types();
        gen.funcs();
    }

    fn export(&mut self, name: &str, iface: &Interface, _files: &mut Files) {
        uwriteln!(self.src, "# Export interface `{name}`\n");
        let mut gen = self.interface(iface);
        gen.types();
        gen.funcs();
    }

    fn export_default(&mut self, name: &str, iface: &Interface, _files: &mut Files) {
        uwriteln!(self.src, "# Default exported interface of `{name}`\n");
        let mut gen = self.interface(iface);
        gen.types();
        gen.funcs();
    }

    fn finish(&mut self, name: &str, _interfaces: &ComponentInterfaces, files: &mut Files) {
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

        files.push(&format!("{name}.md"), self.src.as_bytes());
        files.push(&format!("{name}.html"), html_output.as_bytes());
    }
}

impl Markdown {
    fn interface<'a>(&'a mut self, iface: &'a Interface) -> InterfaceGenerator<'_> {
        let mut sizes = SizeAlign::default();
        sizes.fill(iface);
        InterfaceGenerator {
            gen: self,
            iface,
            sizes,
            types_header_printed: false,
        }
    }
}

struct InterfaceGenerator<'a> {
    gen: &'a mut Markdown,
    iface: &'a Interface,
    sizes: SizeAlign,
    types_header_printed: bool,
}

impl InterfaceGenerator<'_> {
    fn funcs(&mut self) {
        if self.iface.functions.is_empty() {
            return;
        }
        self.push_str("## Functions\n\n");
        for func in self.iface.functions.iter() {
            self.push_str("----\n\n");
            self.push_str(&format!(
                "#### <a href=\"#{0}\" name=\"{0}\"></a> `",
                func.name.to_snake_case()
            ));
            self.gen
                .hrefs
                .insert(func.name.clone(), format!("#{}", func.name.to_snake_case()));
            self.push_str(&func.name);
            self.push_str("` ");
            self.push_str("\n\n");
            self.docs(&func.docs);

            if func.params.len() > 0 {
                self.push_str("##### Params\n\n");
                for (name, ty) in func.params.iter() {
                    self.push_str(&format!(
                        "- <a href=\"#{f}.{p}\" name=\"{f}.{p}\"></a> `{}`: ",
                        name,
                        f = func.name.to_snake_case(),
                        p = name.to_snake_case(),
                    ));
                    self.print_ty(ty, false);
                    self.push_str("\n");
                }
            }

            if func.results.len() > 0 {
                self.push_str("##### Results\n\n");
                for (i, ty) in func.results.iter_types().enumerate() {
                    self.push_str(&format!(
                        "- <a href=\"#{f}.{p}{i}\" name=\"{f}.{p}{i}\"></a> `{}{i}`: ",
                        "result",
                        f = func.name.to_snake_case(),
                        p = "result",
                    ));
                    self.print_ty(ty, false);
                    self.push_str("\n");
                }
            }

            self.push_str("\n");
        }
    }

    fn push_str(&mut self, s: &str) {
        self.gen.src.push_str(s);
    }

    fn print_ty(&mut self, ty: &Type, skip_name: bool) {
        match ty {
            Type::Bool => self.push_str("`bool`"),
            Type::U8 => self.push_str("`u8`"),
            Type::S8 => self.push_str("`s8`"),
            Type::U16 => self.push_str("`u16`"),
            Type::S16 => self.push_str("`s16`"),
            Type::U32 => self.push_str("`u32`"),
            Type::S32 => self.push_str("`s32`"),
            Type::U64 => self.push_str("`u64`"),
            Type::S64 => self.push_str("`s64`"),
            Type::Float32 => self.push_str("`float32`"),
            Type::Float64 => self.push_str("`float64`"),
            Type::Char => self.push_str("`char`"),
            Type::String => self.push_str("`string`"),
            Type::Id(id) => {
                let ty = &self.iface.types[*id];
                if !skip_name {
                    if let Some(name) = &ty.name {
                        self.push_str("[`");
                        self.push_str(name);
                        self.push_str("`](#");
                        self.push_str(&name.to_snake_case());
                        self.push_str(")");
                        return;
                    }
                }
                match &ty.kind {
                    TypeDefKind::Type(t) => self.print_ty(t, false),
                    TypeDefKind::Tuple(t) => {
                        self.push_str("(");
                        for (i, t) in t.types.iter().enumerate() {
                            if i > 0 {
                                self.push_str(", ");
                            }
                            self.print_ty(t, false);
                        }
                        self.push_str(")");
                    }
                    TypeDefKind::Record(_)
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Enum(_)
                    | TypeDefKind::Variant(_)
                    | TypeDefKind::Union(_) => {
                        unreachable!()
                    }
                    TypeDefKind::Option(t) => {
                        self.push_str("option<");
                        self.print_ty(t, false);
                        self.push_str(">");
                    }
                    TypeDefKind::Result(r) => match (r.ok, r.err) {
                        (Some(ok), Some(err)) => {
                            self.push_str("result<");
                            self.print_ty(&ok, false);
                            self.push_str(", ");
                            self.print_ty(&err, false);
                            self.push_str(">");
                        }
                        (None, Some(err)) => {
                            self.push_str("result<_, ");
                            self.print_ty(&err, false);
                            self.push_str(">");
                        }
                        (Some(ok), None) => {
                            self.push_str("result<");
                            self.print_ty(&ok, false);
                            self.push_str(">");
                        }
                        (None, None) => {
                            self.push_str("result");
                        }
                    },
                    TypeDefKind::List(t) => {
                        self.push_str("list<");
                        self.print_ty(t, false);
                        self.push_str(">");
                    }
                    TypeDefKind::Future(t) => match t {
                        Some(t) => {
                            self.push_str("future<");
                            self.print_ty(t, false);
                            self.push_str(">");
                        }
                        None => {
                            self.push_str("future");
                        }
                    },
                    TypeDefKind::Stream(s) => match (s.element, s.end) {
                        (Some(element), Some(end)) => {
                            self.push_str("stream<");
                            self.print_ty(&element, false);
                            self.push_str(", ");
                            self.print_ty(&end, false);
                            self.push_str(">");
                        }
                        (None, Some(end)) => {
                            self.push_str("stream<_, ");
                            self.print_ty(&end, false);
                            self.push_str(">");
                        }
                        (Some(element), None) => {
                            self.push_str("stream<");
                            self.print_ty(&element, false);
                            self.push_str(">");
                        }
                        (None, None) => {
                            self.push_str("stream");
                        }
                    },
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
            self.push_str(line.trim());
            self.push_str("\n");
        }
    }

    fn print_type_header(&mut self, name: &str) {
        if !self.types_header_printed {
            self.push_str("## Types\n\n");
            self.types_header_printed = true;
        }
        self.push_str(&format!(
            "## <a href=\"#{}\" name=\"{0}\"></a> `{}`: ",
            name.to_snake_case(),
            name,
        ));
        self.gen
            .hrefs
            .insert(name.to_string(), format!("#{}", name.to_snake_case()));
    }

    fn print_type_info(&mut self, ty: TypeId, docs: &Docs) {
        self.docs(docs);
        self.push_str("\n");
        self.push_str(&format!("Size: {}, ", self.sizes.size(&Type::Id(ty))));
        self.push_str(&format!("Alignment: {}\n", self.sizes.align(&Type::Id(ty))));
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn iface(&self) -> &'a Interface {
        self.iface
    }

    fn type_record(&mut self, id: TypeId, name: &str, record: &Record, docs: &Docs) {
        self.print_type_header(name);
        self.push_str("record\n\n");
        self.print_type_info(id, docs);
        self.push_str("\n### Record Fields\n\n");
        for field in record.fields.iter() {
            self.push_str(&format!(
                "- <a href=\"{r}.{f}\" name=\"{r}.{f}\"></a> [`{name}`](#{r}.{f}): ",
                r = name.to_snake_case(),
                f = field.name.to_snake_case(),
                name = field.name,
            ));
            self.gen.hrefs.insert(
                format!("{}::{}", name, field.name),
                format!("#{}.{}", name.to_snake_case(), field.name.to_snake_case()),
            );
            self.print_ty(&field.ty, false);
            self.gen.src.indent(1);
            self.push_str("\n\n");
            self.docs(&field.docs);
            self.gen.src.deindent(1);
            self.push_str("\n");
        }
    }

    fn type_tuple(&mut self, id: TypeId, name: &str, tuple: &Tuple, docs: &Docs) {
        self.print_type_header(name);
        self.push_str("tuple\n\n");
        self.print_type_info(id, docs);
        self.push_str("\n### Tuple Fields\n\n");
        for (i, ty) in tuple.types.iter().enumerate() {
            self.push_str(&format!(
                "- <a href=\"{r}.{f}\" name=\"{r}.{f}\"></a> [`{name}`](#{r}.{f}): ",
                r = name.to_snake_case(),
                f = i,
                name = i,
            ));
            self.gen.hrefs.insert(
                format!("{}::{}", name, i),
                format!("#{}.{}", name.to_snake_case(), i),
            );
            self.print_ty(ty, false);
            self.push_str("\n");
        }
    }

    fn type_flags(&mut self, id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        self.print_type_header(name);
        self.push_str("record\n\n");
        self.print_type_info(id, docs);
        self.push_str("\n### Record Fields\n\n");
        for (i, flag) in flags.flags.iter().enumerate() {
            self.push_str(&format!(
                "- <a href=\"{r}.{f}\" name=\"{r}.{f}\"></a> [`{name}`](#{r}.{f}): ",
                r = name.to_snake_case(),
                f = flag.name.to_snake_case(),
                name = flag.name,
            ));
            self.gen.hrefs.insert(
                format!("{}::{}", name, flag.name),
                format!("#{}.{}", name.to_snake_case(), flag.name.to_snake_case()),
            );
            self.gen.src.indent(1);
            self.push_str("\n\n");
            self.docs(&flag.docs);
            self.gen.src.deindent(1);
            self.push_str(&format!("Bit: {}\n", i));
            self.push_str("\n");
        }
    }

    fn type_variant(&mut self, id: TypeId, name: &str, variant: &Variant, docs: &Docs) {
        self.print_type_header(name);
        self.push_str("variant\n\n");
        self.print_type_info(id, docs);
        self.push_str("\n### Variant Cases\n\n");
        for case in variant.cases.iter() {
            self.push_str(&format!(
                "- <a href=\"{v}.{c}\" name=\"{v}.{c}\"></a> [`{name}`](#{v}.{c})",
                v = name.to_snake_case(),
                c = case.name.to_snake_case(),
                name = case.name,
            ));
            self.gen.hrefs.insert(
                format!("{}::{}", name, case.name),
                format!("#{}.{}", name.to_snake_case(), case.name.to_snake_case()),
            );
            if let Some(ty) = &case.ty {
                self.push_str(": ");
                self.print_ty(ty, false);
            }
            self.gen.src.indent(1);
            self.push_str("\n\n");
            self.docs(&case.docs);
            self.gen.src.deindent(1);
            self.push_str("\n");
        }
    }

    fn type_union(&mut self, id: TypeId, name: &str, union: &Union, docs: &Docs) {
        self.print_type_header(name);
        self.push_str("union\n\n");
        self.print_type_info(id, docs);
        self.push_str("\n### Union Cases\n\n");
        let snake = name.to_snake_case();
        for (i, case) in union.cases.iter().enumerate() {
            self.push_str(&format!(
                "- <a href=\"{snake}.{i}\" name=\"{snake}.{i}\"></a> [`{i}`](#{snake}.{i})",
            ));
            self.gen
                .hrefs
                .insert(format!("{name}::{i}"), format!("#{snake}.{i}"));
            self.push_str(": ");
            self.print_ty(&case.ty, false);
            self.gen.src.indent(1);
            self.push_str("\n\n");
            self.docs(&case.docs);
            self.gen.src.deindent(1);
            self.push_str("\n");
        }
    }

    fn type_enum(&mut self, id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        self.print_type_header(name);
        self.push_str("enum\n\n");
        self.print_type_info(id, docs);
        self.push_str("\n### Enum Cases\n\n");
        for case in enum_.cases.iter() {
            self.push_str(&format!(
                "- <a href=\"{v}.{c}\" name=\"{v}.{c}\"></a> [`{name}`](#{v}.{c})",
                v = name.to_snake_case(),
                c = case.name.to_snake_case(),
                name = case.name,
            ));
            self.gen.hrefs.insert(
                format!("{}::{}", name, case.name),
                format!("#{}.{}", name.to_snake_case(), case.name.to_snake_case()),
            );
            self.gen.src.indent(1);
            self.push_str("\n\n");
            self.docs(&case.docs);
            self.gen.src.deindent(1);
            self.push_str("\n");
        }
    }

    fn type_option(&mut self, id: TypeId, name: &str, payload: &Type, docs: &Docs) {
        self.print_type_header(name);
        self.push_str("option<");
        self.print_ty(payload, false);
        self.push_str(">");
        self.print_type_info(id, docs);
    }

    fn type_result(&mut self, id: TypeId, name: &str, result: &Result_, docs: &Docs) {
        self.print_type_header(name);
        match (result.ok, result.err) {
            (Some(ok), Some(err)) => {
                self.push_str("result<");
                self.print_ty(&ok, false);
                self.push_str(", ");
                self.print_ty(&err, false);
                self.push_str(">");
            }
            (None, Some(err)) => {
                self.push_str("result<_, ");
                self.print_ty(&err, false);
                self.push_str(">");
            }
            (Some(ok), None) => {
                self.push_str("result<");
                self.print_ty(&ok, false);
                self.push_str(">");
            }
            (None, None) => {
                self.push_str("result");
            }
        }
        self.print_type_info(id, docs);
    }

    fn type_alias(&mut self, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.print_type_header(name);
        self.print_ty(ty, true);
        self.push_str("\n\n");
        self.print_type_info(id, docs);
        self.push_str("\n");
    }

    fn type_list(&mut self, id: TypeId, name: &str, _ty: &Type, docs: &Docs) {
        self.type_alias(id, name, &Type::Id(id), docs);
    }

    fn type_builtin(&mut self, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.type_alias(id, name, ty, docs)
    }
}
