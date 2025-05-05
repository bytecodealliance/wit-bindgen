use anyhow::Result;
use heck::*;
use pulldown_cmark::{html, Event, LinkType, Parser, Tag};
use std::collections::HashMap;
use std::fmt::Write;
use wit_bindgen_core::{
    uwriteln, wit_parser, Files, InterfaceGenerator as _, Source, WorldGenerator,
};
use wit_parser::*;

#[derive(Default)]
struct Markdown {
    src: Source,
    opts: Opts,
    hrefs: HashMap<String, String>,
    sizes: SizeAlign,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Output a `.md` file containing HTML.
    ///
    /// This can be useful when producing files to be displayed on Github,
    /// as it doesn't render HTML files, but it does render Markdown files,
    /// which can contain HTML.
    #[cfg_attr(feature = "clap", arg(long))]
    html_in_md: bool,
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        let mut r = Markdown::default();
        r.opts = self.clone();
        Box::new(r)
    }
}

impl WorldGenerator for Markdown {
    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        self.sizes.fill(resolve);

        let world = &resolve.worlds[world];
        uwriteln!(
            self.src,
            "# <a id=\"{}\"></a>World {}\n",
            world.name.to_snake_case(),
            world.name
        );
        self.hrefs.insert(
            world.name.to_string(),
            format!("#{}", world.name.to_snake_case()),
        );

        let mut gen = self.interface(resolve);

        gen.docs(&world.docs);
        gen.push_str("\n");

        // Produce a table of contents for the world.
        let mut first = true;
        for (name, import) in &world.imports {
            if first {
                gen.push_str(" - Imports:\n");
                first = false;
            }
            let name = &resolve.name_world_key(name);
            match import {
                WorldItem::Interface { .. } => {
                    gen.push_str("    - interface `");
                    gen.push_str(name);
                    gen.push_str("`\n");
                }
                WorldItem::Function(_) => {
                    gen.push_str("    - function `");
                    gen.push_str(name);
                    gen.push_str("`\n");
                }
                WorldItem::Type(_) => {
                    gen.push_str("    - type `");
                    gen.push_str(name);
                    gen.push_str("`\n");
                }
            }
        }
        let mut first = true;
        for (name, export) in &world.exports {
            if first {
                gen.push_str(" - Exports:\n");
                first = false;
            }
            let name = &resolve.name_world_key(name);
            match export {
                WorldItem::Interface { .. } => {
                    gen.push_str("    - interface `");
                    gen.push_str(name);
                    gen.push_str("`\n");
                }
                WorldItem::Function(_) => {
                    gen.push_str("    - function `");
                    gen.push_str(name);
                    gen.push_str("`\n");
                }
                WorldItem::Type(_) => {
                    gen.push_str("    - type `");
                    gen.push_str(name);
                    gen.push_str("`\n");
                }
            }
        }
        gen.push_str("\n");
    }

    fn import_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        let name = resolve.name_world_key(name);
        uwriteln!(
            self.src,
            "## <a id=\"{}\"></a>Import interface {name}\n",
            name.to_snake_case()
        );
        self.hrefs
            .insert(name.to_string(), format!("#{}", name.to_snake_case()));
        let mut gen = self.interface(resolve);
        gen.docs(&resolve.interfaces[id].docs);
        gen.push_str("\n");
        gen.types(id);
        gen.funcs(id);

        Ok(())
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let name = &resolve.worlds[world].name;
        uwriteln!(self.src, "## Imported functions to world `{name}`\n");
        let mut gen = self.interface(resolve);
        for (_, func) in funcs {
            gen.func(func);
        }
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        let name = resolve.name_world_key(name);
        uwriteln!(
            self.src,
            "## <a id=\"{}\"></a>Export interface {name}\n",
            name.to_snake_case()
        );
        self.hrefs
            .insert(name.to_string(), format!("#{}", name.to_snake_case()));
        let mut gen = self.interface(resolve);
        gen.types(id);
        gen.funcs(id);
        Ok(())
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> Result<()> {
        let name = &resolve.worlds[world].name;
        uwriteln!(self.src, "## Exported functions from world `{name}`\n");
        let mut gen = self.interface(resolve);
        for (_, func) in funcs {
            gen.func(func);
        }
        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let name = &resolve.worlds[world].name;
        uwriteln!(self.src, "## Exported types from world `{name}`\n");
        let mut gen = self.interface(resolve);
        for (name, ty) in types {
            gen.define_type(name, *ty);
        }
    }

    fn finish(&mut self, resolve: &Resolve, world: WorldId, files: &mut Files) -> Result<()> {
        let world = &resolve.worlds[world];
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

        if self.opts.html_in_md {
            // Write the html output into a .md file.
            files.push(&format!("{}.md", world.name), html_output.as_bytes());
        } else {
            // Write the html output to an html file, and md output to a md file.
            files.push(&format!("{}.md", world.name), self.src.as_bytes());
            files.push(&format!("{}.html", world.name), html_output.as_bytes());
        }

        Ok(())
    }
}

impl Markdown {
    fn interface<'a>(&'a mut self, resolve: &'a Resolve) -> InterfaceGenerator<'a> {
        InterfaceGenerator {
            gen: self,
            resolve,
            types_header_printed: false,
        }
    }
}

struct InterfaceGenerator<'a> {
    gen: &'a mut Markdown,
    resolve: &'a Resolve,
    types_header_printed: bool,
}

impl InterfaceGenerator<'_> {
    fn funcs(&mut self, id: InterfaceId) {
        let iface = &self.resolve.interfaces[id];
        if iface.functions.is_empty() {
            return;
        }
        self.push_str("----\n\n");
        self.push_str("### Functions\n\n");
        for (_name, func) in iface.functions.iter() {
            self.func(func);
        }
    }

    fn func(&mut self, func: &Function) {
        self.push_str(&format!(
            "#### <a id=\"{0}\"></a>`",
            func.name.to_snake_case()
        ));
        self.gen
            .hrefs
            .insert(func.name.clone(), format!("#{}", func.name.to_snake_case()));
        self.push_str(&func.name);
        self.push_str(": func`");
        self.push_str("\n\n");
        self.docs(&func.docs);

        if func.params.len() > 0 {
            self.push_str("\n");
            self.push_str("##### Params\n\n");
            for (name, ty) in func.params.iter() {
                self.push_str(&format!(
                    "- <a id=\"{f}.{p}\"></a>`{}`: ",
                    name,
                    f = func.name.to_snake_case(),
                    p = name.to_snake_case(),
                ));
                self.print_ty(ty);
                self.push_str("\n");
            }
        }

        if let Some(ty) = &func.result {
            self.push_str("\n##### Return values\n\n");
            self.push_str(&format!(
                "- <a id=\"{f}.0\"></a> ",
                f = func.name.to_snake_case(),
            ));
            self.print_ty(ty);
            self.push_str("\n");
        }

        self.push_str("\n");
    }

    fn push_str(&mut self, s: &str) {
        self.gen.src.push_str(s);
    }

    fn print_ty(&mut self, ty: &Type) {
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
            Type::F32 => self.push_str("`f32`"),
            Type::F64 => self.push_str("`f64`"),
            Type::Char => self.push_str("`char`"),
            Type::String => self.push_str("`string`"),
            Type::ErrorContext => self.push_str("`error-context`"),
            Type::Id(id) => {
                let ty = &self.resolve.types[*id];
                if let Some(name) = &ty.name {
                    self.push_str("[`");
                    self.push_str(name);
                    self.push_str("`](#");
                    self.push_str(&name.to_snake_case());
                    self.push_str(")");
                    return;
                }
                match &ty.kind {
                    TypeDefKind::Type(t) => self.print_ty(t),
                    TypeDefKind::Tuple(t) => {
                        self.push_str("(");
                        for (i, t) in t.types.iter().enumerate() {
                            if i > 0 {
                                self.push_str(", ");
                            }
                            self.print_ty(t);
                        }
                        self.push_str(")");
                    }
                    TypeDefKind::Record(_)
                    | TypeDefKind::Resource
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Enum(_)
                    | TypeDefKind::Variant(_) => {
                        // These types are always named, so we will have
                        // printed the name above, so we don't need to print
                        // the contents.
                        assert!(ty.name.is_some());
                    }
                    TypeDefKind::Option(t) => {
                        self.push_str("option<");
                        self.print_ty(t);
                        self.push_str(">");
                    }
                    TypeDefKind::Result(r) => match (r.ok, r.err) {
                        (Some(ok), Some(err)) => {
                            self.push_str("result<");
                            self.print_ty(&ok);
                            self.push_str(", ");
                            self.print_ty(&err);
                            self.push_str(">");
                        }
                        (None, Some(err)) => {
                            self.push_str("result<_, ");
                            self.print_ty(&err);
                            self.push_str(">");
                        }
                        (Some(ok), None) => {
                            self.push_str("result<");
                            self.print_ty(&ok);
                            self.push_str(">");
                        }
                        (None, None) => {
                            self.push_str("result");
                        }
                    },
                    TypeDefKind::List(t) => {
                        self.push_str("list<");
                        self.print_ty(t);
                        self.push_str(">");
                    }
                    TypeDefKind::Future(t) => match t {
                        Some(t) => {
                            self.push_str("future<");
                            self.print_ty(t);
                            self.push_str(">");
                        }
                        None => {
                            self.push_str("future");
                        }
                    },
                    TypeDefKind::Stream(t) => match t {
                        Some(t) => {
                            self.push_str("stream<");
                            self.print_ty(t);
                            self.push_str(">");
                        }
                        None => {
                            self.push_str("stream");
                        }
                    },
                    TypeDefKind::Handle(Handle::Own(ty)) => {
                        self.push_str("own<");
                        self.print_ty(&Type::Id(*ty));
                        self.push_str(">");
                    }
                    TypeDefKind::Handle(Handle::Borrow(ty)) => {
                        self.push_str("borrow<");
                        self.print_ty(&Type::Id(*ty));
                        self.push_str(">");
                    }
                    TypeDefKind::Unknown => unreachable!(),
                    TypeDefKind::FixedSizeList(..) => todo!(),
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

    fn print_type_header(&mut self, type_: &str, name: &str) {
        if !self.types_header_printed {
            self.push_str("----\n\n");
            self.push_str("### Types\n\n");
            self.types_header_printed = true;
        }
        self.push_str(&format!(
            "#### <a id=\"{}\"></a>`{} {}`\n",
            name.to_snake_case(),
            type_,
            name,
        ));
        self.gen
            .hrefs
            .insert(name.to_string(), format!("#{}", name.to_snake_case()));
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn type_record(&mut self, _id: TypeId, name: &str, record: &Record, docs: &Docs) {
        self.print_type_header("record", name);
        self.push_str("\n");
        self.docs(docs);
        self.push_str("\n##### Record Fields\n\n");
        for field in record.fields.iter() {
            self.push_str(&format!(
                "- <a id=\"{r}.{f}\"></a>`{name}`: ",
                r = name.to_snake_case(),
                f = field.name.to_snake_case(),
                name = field.name,
            ));
            self.gen.hrefs.insert(
                format!("{}::{}", name, field.name),
                format!("#{}.{}", name.to_snake_case(), field.name.to_snake_case()),
            );
            self.print_ty(&field.ty);
            if field.docs.contents.is_some() {
                self.gen.src.indent(1);
                self.push_str("\n<p>");
                self.docs(&field.docs);
                self.gen.src.deindent(1);
            }
            self.push_str("\n");
        }
    }

    fn type_resource(&mut self, _id: TypeId, name: &str, docs: &Docs) {
        self.print_type_header("resource", name);
        self.push_str("\n");
        self.docs(docs);
    }

    fn type_tuple(&mut self, _id: TypeId, name: &str, tuple: &Tuple, docs: &Docs) {
        self.print_type_header("tuple", name);
        self.push_str("\n");
        self.docs(docs);
        self.push_str("\n##### Tuple Fields\n\n");
        for (i, ty) in tuple.types.iter().enumerate() {
            self.push_str(&format!(
                "- <a id=\"{r}.{f}\"></a>`{name}`: ",
                r = name.to_snake_case(),
                f = i,
                name = i,
            ));
            self.gen.hrefs.insert(
                format!("{}::{}", name, i),
                format!("#{}.{}", name.to_snake_case(), i),
            );
            self.print_ty(ty);
            self.push_str("\n");
        }
    }

    fn type_flags(&mut self, _id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        self.print_type_header("flags", name);
        self.push_str("\n");
        self.docs(docs);
        self.push_str("\n##### Flags members\n\n");
        for flag in flags.flags.iter() {
            self.push_str(&format!(
                "- <a id=\"{r}.{f}\"></a>`{name}`: ",
                r = name.to_snake_case(),
                f = flag.name.to_snake_case(),
                name = flag.name,
            ));
            self.gen.hrefs.insert(
                format!("{}::{}", name, flag.name),
                format!("#{}.{}", name.to_snake_case(), flag.name.to_snake_case()),
            );
            if flag.docs.contents.is_some() {
                self.gen.src.indent(1);
                self.push_str("\n<p>");
                self.docs(&flag.docs);
                self.gen.src.deindent(1);
            }
            self.push_str("\n");
        }
    }

    fn type_variant(&mut self, _id: TypeId, name: &str, variant: &Variant, docs: &Docs) {
        self.print_type_header("variant", name);
        self.push_str("\n");
        self.docs(docs);
        self.push_str("\n##### Variant Cases\n\n");
        for case in variant.cases.iter() {
            self.push_str(&format!(
                "- <a id=\"{v}.{c}\"></a>`{name}`",
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
                self.print_ty(ty);
            }
            if case.docs.contents.is_some() {
                self.gen.src.indent(1);
                self.push_str("\n<p>");
                self.docs(&case.docs);
                self.gen.src.deindent(1);
            }
            self.push_str("\n");
        }
    }

    fn type_enum(&mut self, _id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        self.print_type_header("enum", name);
        self.push_str("\n");
        self.docs(docs);
        self.push_str("\n##### Enum Cases\n\n");
        for case in enum_.cases.iter() {
            self.push_str(&format!(
                "- <a id=\"{v}.{c}\"></a>`{name}`",
                v = name.to_snake_case(),
                c = case.name.to_snake_case(),
                name = case.name,
            ));
            self.gen.hrefs.insert(
                format!("{}::{}", name, case.name),
                format!("#{}.{}", name.to_snake_case(), case.name.to_snake_case()),
            );
            if case.docs.contents.is_some() {
                self.gen.src.indent(1);
                self.push_str("\n<p>");
                self.docs(&case.docs);
                self.gen.src.deindent(1);
            }
            self.push_str("\n");
        }
    }

    fn type_option(&mut self, _id: TypeId, name: &str, payload: &Type, docs: &Docs) {
        self.print_type_header("type", name);
        self.push_str("option<");
        self.print_ty(payload);
        self.push_str(">");
        self.push_str("\n");
        self.docs(docs);
    }

    fn type_result(&mut self, _id: TypeId, name: &str, result: &Result_, docs: &Docs) {
        self.print_type_header("type", name);
        match (result.ok, result.err) {
            (Some(ok), Some(err)) => {
                self.push_str("result<");
                self.print_ty(&ok);
                self.push_str(", ");
                self.print_ty(&err);
                self.push_str(">");
            }
            (None, Some(err)) => {
                self.push_str("result<_, ");
                self.print_ty(&err);
                self.push_str(">");
            }
            (Some(ok), None) => {
                self.push_str("result<");
                self.print_ty(&ok);
                self.push_str(">");
            }
            (None, None) => {
                self.push_str("result");
            }
        }
        self.push_str("\n");
        self.docs(docs);
    }

    fn type_alias(&mut self, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.print_type_header("type", name);
        self.print_ty(ty);
        self.push_str("\n<p>");
        self.docs(docs);
        self.push_str("\n");
    }

    fn type_list(&mut self, id: TypeId, name: &str, _ty: &Type, docs: &Docs) {
        self.type_alias(id, name, &Type::Id(id), docs);
    }

    fn type_future(&mut self, id: TypeId, name: &str, ty: &Option<Type>, docs: &Docs) {
        _ = (id, name, ty, docs);
        todo!()
    }

    fn type_stream(&mut self, id: TypeId, name: &str, ty: &Option<Type>, docs: &Docs) {
        _ = (id, name, ty, docs);
        todo!()
    }

    fn type_builtin(&mut self, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.type_alias(id, name, ty, docs)
    }
}
