use heck::*;
use pulldown_cmark::{html, Event, LinkType, Parser, Tag};
use std::collections::HashMap;
use wit_bindgen_gen_core::{wit_parser, Direction, Files, Generator, Source};
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
                    TypeDefKind::Record(r) => {
                        assert!(r.is_tuple());
                        self.src.push_str("(");
                        for (i, f) in r.fields.iter().enumerate() {
                            if i > 0 {
                                self.src.push_str(", ");
                            }
                            self.print_ty(iface, &f.ty, false);
                        }
                        self.src.push_str(")");
                    }
                    TypeDefKind::Variant(v) => {
                        if v.is_bool() {
                            self.src.push_str("`bool`");
                        } else if let Some(t) = v.as_option() {
                            self.src.push_str("option<");
                            self.print_ty(iface, t, false);
                            self.src.push_str(">");
                        } else if let Some((ok, err)) = v.as_expected() {
                            self.src.push_str("expected<");
                            match ok {
                                Some(t) => self.print_ty(iface, t, false),
                                None => self.src.push_str("_"),
                            }
                            self.src.push_str(", ");
                            match err {
                                Some(t) => self.print_ty(iface, t, false),
                                None => self.src.push_str("_"),
                            }
                            self.src.push_str(">");
                        } else {
                            unreachable!()
                        }
                    }
                    TypeDefKind::List(Type::Char) => self.src.push_str("`string`"),
                    TypeDefKind::List(t) => {
                        self.src.push_str("list<");
                        self.print_ty(iface, t, false);
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
        for (i, field) in record.fields.iter().enumerate() {
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
            if record.is_flags() {
                self.src.push_str(&format!("Bit: {}\n", i));
            }
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
            if let Some(ty) = &case.ty {
                self.src.push_str(": ");
                self.print_ty(iface, ty, false);
            }
            self.src.indent(1);
            self.src.push_str("\n\n");
            self.docs(&case.docs);
            self.src.deindent(1);
            self.src.push_str("\n");
        }
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
        if func.results.len() > 0 {
            self.src.push_str("##### Results\n\n");
            for (name, ty) in func.results.iter() {
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
