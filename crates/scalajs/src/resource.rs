use crate::context::{write_doc_comment, EncodedName, OwnerContext, ScalaJsContext};
use crate::interface::ScalaJsInterface;
use heck::{ToLowerCamelCase, ToPascalCase};
use std::fmt::Write;
use wit_bindgen_core::wit_parser::{Function, FunctionKind, Resolve, TypeId};
use wit_bindgen_core::{uwrite, uwriteln};

pub struct ScalaJsImportedResource {
    _resource_id: TypeId,
    resource_name: String,
    class_header: String,
    class_source: String,
    object_source: String,
    constructor_args: String,
    pub name: EncodedName,
    indent: String,
    annotations: Vec<String>,
}

impl ScalaJsImportedResource {
    pub fn new(
        context: &ScalaJsContext,
        resolve: &Resolve,
        resource_id: TypeId,
        indent: &str,
    ) -> Self {
        let resource = &resolve.types[resource_id];
        let resource_name = resource
            .name
            .clone()
            .expect("Anonymous resources not supported");
        let encoded_resource_name = context.encode_name(resource_name.to_pascal_case());

        let mut class_header = String::new();
        write_doc_comment(&mut class_header, "    ", &resource.docs);

        uwriteln!(class_header, "{indent}@js.native");
        encoded_resource_name.write_rename_attribute(&mut class_header, "    ");
        uwrite!(
            class_header,
            "{indent}class {}(",
            encoded_resource_name.scala
        );

        let mut class_source = String::new();
        uwriteln!(class_source, ") extends js.Object {{");

        let mut object_source = String::new();
        uwriteln!(object_source, "{indent}@js.native");
        uwriteln!(
            object_source,
            "{indent}object {} extends js.Object {{",
            encoded_resource_name.scala
        );

        Self {
            _resource_id: resource_id,
            resource_name,
            class_header,
            class_source,
            object_source,
            constructor_args: String::new(),
            name: encoded_resource_name,
            indent: indent.to_string(),
            annotations: Vec::new(),
        }
    }

    pub fn annotate(&mut self, annotation: &str) {
        self.annotations.push(annotation.to_string());
    }

    pub fn add_function(
        &mut self,
        context: &ScalaJsContext,
        resolve: &Resolve,
        owner_context: &impl OwnerContext,
        func_name: &str,
        func: &Function,
    ) {
        match &func.kind {
            FunctionKind::Freestanding => unreachable!(),
            FunctionKind::Method(_) => {
                let args = context.render_args(owner_context, resolve, func.params.iter().skip(1));
                let ret = context.render_return_type(owner_context, resolve, &func.results);
                let encoded_func_name = self.get_func_name(context, "[method]", func_name);

                let overrd = if context
                    .keywords
                    .base_methods
                    .contains(&encoded_func_name.scala)
                {
                    "override "
                } else {
                    ""
                };

                write_doc_comment(
                    &mut self.class_source,
                    &format!("{}  ", self.indent),
                    &func.docs,
                );
                encoded_func_name.write_rename_attribute(&mut self.class_source, "      ");
                uwriteln!(
                    self.class_source,
                    "{}  {overrd}def {}({args}): {ret} = js.native",
                    self.indent,
                    encoded_func_name.scala
                );
            }
            FunctionKind::Static(_) => {
                let args = context.render_args(owner_context, resolve, func.params.iter());
                let ret = context.render_return_type(owner_context, resolve, &func.results);

                let encoded_func_name = self.get_func_name(context, "[static]", func_name);
                write_doc_comment(&mut self.object_source, "      ", &func.docs);
                encoded_func_name
                    .write_rename_attribute(&mut self.object_source, &format!("{}  ", self.indent));
                uwriteln!(
                    self.object_source,
                    "{}  def {}({args}): {ret} = js.native",
                    self.indent,
                    encoded_func_name.scala
                );
            }
            FunctionKind::Constructor(_) => {
                let args = context.render_args(owner_context, resolve, func.params.iter());
                self.constructor_args = args;
            }
        }
    }

    pub fn finalize(self) -> String {
        let mut class_source = String::new();
        for annotation in &self.annotations {
            uwriteln!(class_source, "{}{}", self.indent, annotation);
        }
        uwriteln!(class_source, "{}", self.class_header);
        uwrite!(class_source, "{}", self.constructor_args);
        uwriteln!(class_source, "{}", self.class_source);
        uwriteln!(class_source, "{}}}", self.indent);
        let mut object_source = String::new();
        for annotation in self.annotations {
            uwriteln!(object_source, "{}{}", self.indent, annotation);
        }
        uwriteln!(object_source, "{}", self.object_source);
        uwriteln!(object_source, "{}}}", self.indent);
        format!("{}\n{}\n", class_source, object_source)
    }

    fn get_func_name(
        &self,
        context: &ScalaJsContext,
        prefix: &str,
        func_name: &str,
    ) -> EncodedName {
        let name = func_name
            .strip_prefix(prefix)
            .unwrap()
            .strip_prefix(&self.resource_name)
            .unwrap()
            .to_lower_camel_case();
        context.encode_name(name)
    }
}

pub struct ScalaJsExportedResource<'a> {
    owner: &'a ScalaJsInterface<'a>,
    _resource_id: TypeId,
    resource_name: String,
    class_header: String,
    class_source: String,
    static_methods: String,
    constructor_args: String,
}

impl<'a> ScalaJsExportedResource<'a> {
    pub fn new(owner: &'a ScalaJsInterface<'a>, resource_id: TypeId) -> Self {
        let resource = &owner.resolve.types[resource_id];
        let resource_name = resource
            .name
            .clone()
            .expect("Anonymous resources not supported");
        let encoded_resource_name = owner
            .generator
            .context
            .encode_name(resource_name.to_pascal_case());

        let mut class_header = String::new();
        write_doc_comment(&mut class_header, "    ", &resource.docs);

        encoded_resource_name.write_rename_attribute(&mut class_header, "    // ");
        uwrite!(
            class_header,
            "  abstract class {}(",
            encoded_resource_name.scala
        );

        let mut class_source = String::new();
        uwriteln!(class_source, ") extends js.Object {{");

        Self {
            owner,
            _resource_id: resource_id,
            resource_name,
            class_header,
            class_source,
            static_methods: String::new(),
            constructor_args: String::new(),
        }
    }

    pub fn add_function(&mut self, func_name: &str, func: &Function) {
        match &func.kind {
            FunctionKind::Freestanding => unreachable!(),
            FunctionKind::Method(_) => {
                let args = self.owner.generator.context.render_args(
                    self.owner,
                    self.owner.resolve,
                    func.params.iter().skip(1),
                );
                let ret = self.owner.generator.context.render_return_type(
                    self.owner,
                    self.owner.resolve,
                    &func.results,
                );
                let encoded_func_name = self.get_func_name("[method]", func_name);

                let overrd = if self
                    .owner
                    .generator
                    .context
                    .keywords
                    .base_methods
                    .contains(&encoded_func_name.scala)
                {
                    "override "
                } else {
                    ""
                };

                write_doc_comment(&mut self.class_source, "      ", &func.docs);
                encoded_func_name.write_rename_attribute(&mut self.class_source, "      ");
                uwriteln!(
                    self.class_source,
                    "    {overrd}def {}({args}): {ret}",
                    encoded_func_name.scala
                );
            }
            FunctionKind::Static(_) => {
                let args = self.owner.generator.context.render_args(
                    self.owner,
                    self.owner.resolve,
                    func.params.iter(),
                );
                let ret = self.owner.generator.context.render_return_type(
                    self.owner,
                    self.owner.resolve,
                    &func.results,
                );

                let encoded_func_name = self.get_func_name("[static]", func_name);
                write_doc_comment(&mut self.static_methods, "      ", &func.docs);
                uwriteln!(self.static_methods, "    // @JSExportStatic");
                encoded_func_name.write_rename_attribute(&mut self.static_methods, "    ");
                uwriteln!(
                    self.static_methods,
                    "    def {}({args}): {ret}",
                    encoded_func_name.scala
                );
            }
            FunctionKind::Constructor(_) => {
                let args = self.owner.generator.context.render_args(
                    self.owner,
                    self.owner.resolve,
                    func.params.iter(),
                );
                self.constructor_args = args;
            }
        }
    }

    pub fn finalize(self) -> String {
        let mut class_source = self.class_header;
        uwrite!(class_source, "{}", self.constructor_args);
        uwriteln!(class_source, "{}", self.class_source);
        uwriteln!(class_source, "  }}");
        uwriteln!(class_source, "");

        let scala_resource_name = self
            .owner
            .generator
            .context
            .keywords
            .escape(self.resource_name.to_pascal_case());
        uwriteln!(class_source, "  trait {}Static {{", scala_resource_name);
        uwriteln!(class_source, "{}", self.static_methods);
        uwriteln!(class_source, "  }}");
        class_source
    }

    fn get_func_name(&self, prefix: &str, func_name: &str) -> EncodedName {
        let name = func_name
            .strip_prefix(prefix)
            .unwrap()
            .strip_prefix(&self.resource_name)
            .unwrap()
            .to_lower_camel_case();
        self.owner.generator.context.encode_name(name)
    }
}
