use crate::context::{package_name_to_segments, write_doc_comment, ScalaJsFile};
use crate::resource::{ScalaJsExportedResource, ScalaJsImportedResource};
use crate::ScalaJs;
use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use std::collections::HashMap;
use std::fmt::Write;
use wit_bindgen_core::wit_parser::{
    FunctionKind, Interface, InterfaceId, Resolve, TypeDefKind, TypeId,
};
use wit_bindgen_core::Direction::{Export, Import};
use wit_bindgen_core::{uwriteln, Direction};

pub struct ScalaJsInterface<'a> {
    wit_name: String,
    pub name: String,
    package_object_name: String,
    source: String,
    package: Vec<String>,
    pub resolve: &'a Resolve,
    interface: &'a Interface,
    pub interface_id: InterfaceId,
    pub direction: Direction,
    pub generator: &'a mut ScalaJs,
}

impl<'a> ScalaJsInterface<'a> {
    pub fn new(
        wit_name: String,
        resolve: &'a Resolve,
        interface_id: InterfaceId,
        direction: Direction,
        generator: &'a mut ScalaJs,
    ) -> Self {
        let interface = &resolve.interfaces[interface_id];
        let name = interface
            .name
            .clone()
            .unwrap_or(wit_name.clone())
            .to_pascal_case();

        let package_name = resolve.packages
            [interface.package.expect("missing package for interface")]
        .name
        .clone();

        let package = package_name_to_segments(
            &generator.context.opts,
            &package_name,
            &direction,
            &generator.context.keywords,
            false,
        );

        let package_object_name = generator.context.keywords.escape(name.to_snake_case());

        Self {
            wit_name,
            name,
            package_object_name,
            source: "".to_string(),
            package,
            resolve,
            interface,
            interface_id,
            direction,
            generator,
        }
    }

    pub fn generate(&mut self) {
        match self.direction {
            Import => self.generate_import(),
            Export => self.generate_export(),
        }
    }

    pub fn generate_import(&mut self) {
        let mut source = String::new();
        self.generate_package_header(&mut source);

        let types = self.collect_type_definition_snippets();
        let imported_resources = self.collect_imported_resources();
        let functions = self.collect_function_snippets();

        for typ in types {
            uwriteln!(source, "{}", typ);
            uwriteln!(source, "");
        }

        write_doc_comment(&mut source, "  ", &self.interface.docs);
        uwriteln!(source, "  @js.native");
        uwriteln!(source, "  trait {} extends js.Object {{", self.name);

        for (_, resource) in imported_resources {
            uwriteln!(source, "{}", resource.finalize());
            uwriteln!(source, "");
        }

        for function in functions {
            uwriteln!(source, "{function}");
        }

        uwriteln!(source, "  }}");

        uwriteln!(source, "");
        uwriteln!(source, "  @js.native");
        uwriteln!(
            source,
            "  @JSImport(\"{}\", JSImport.Namespace)",
            self.wit_name
        );
        uwriteln!(source, "  object {} extends {}", self.name, self.name);

        uwriteln!(source, "}}");
        self.source = source;
    }

    pub fn generate_export(&mut self) {
        let mut source = String::new();
        self.generate_package_header(&mut source);

        let also_imported = self
            .generator
            .context
            .interface_direction(&self.interface_id)
            == Import;

        let types = if also_imported {
            Vec::new()
        } else {
            self.collect_type_definition_snippets()
        };
        let exported_resources = self.collect_exported_resources();
        let functions = self.collect_function_snippets();

        for typ in types {
            uwriteln!(source, "{}", typ);
            uwriteln!(source, "");
        }

        for (_, resource) in exported_resources {
            uwriteln!(source, "{}", resource.finalize());
            uwriteln!(source, "");
        }

        write_doc_comment(&mut source, "  ", &self.interface.docs);
        uwriteln!(source, "  trait {} {{", self.name);
        for function in functions {
            uwriteln!(source, "{function}");
        }

        uwriteln!(source, "  }}");

        uwriteln!(source, "}}");
        self.source = source;
    }

    fn generate_package_header(&mut self, source: &mut String) {
        uwriteln!(source, "package {}", self.package.join("."));
        uwriteln!(source, "");
        uwriteln!(source, "import scala.scalajs.js");
        uwriteln!(source, "import scala.scalajs.js.annotation._");
        uwriteln!(
            source,
            "import {}wit._",
            self.generator.context.opts.base_package_prefix()
        );
        uwriteln!(source, "");

        uwriteln!(source, "package object {} {{", self.package_object_name);
    }

    fn collect_type_definition_snippets(&mut self) -> Vec<String> {
        let mut types = Vec::new();

        for (type_name, type_id) in &self.interface.types {
            let type_def = &self.resolve.types[*type_id];

            let type_name = self
                .generator
                .context
                .overrides
                .get(type_id)
                .unwrap_or(type_name);
            let type_name = if type_name.eq_ignore_ascii_case(&self.name) {
                let overridden_type_name = format!("{}Type", type_name);
                self.generator
                    .context
                    .overrides
                    .insert(*type_id, overridden_type_name.clone());
                overridden_type_name
            } else {
                type_name.clone()
            };

            if let Some(typ) =
                self.generator
                    .context
                    .render_typedef(self, &self.resolve, &type_name, type_def)
            {
                types.push(typ);
            }
        }

        types
    }

    fn collect_imported_resources(&self) -> HashMap<TypeId, ScalaJsImportedResource> {
        let mut imported_resources = HashMap::new();
        for (_, type_id) in &self.interface.types {
            let type_def = &self.resolve.types[*type_id];
            if let TypeDefKind::Resource = &type_def.kind {
                imported_resources.entry(*type_id).or_insert_with(|| {
                    ScalaJsImportedResource::new(
                        &self.generator.context,
                        self.resolve,
                        *type_id,
                        "    ",
                    )
                });
            }
        }

        for (func_name, func) in &self.interface.functions {
            match func.kind {
                FunctionKind::Method(resource_type)
                | FunctionKind::Static(resource_type)
                | FunctionKind::Constructor(resource_type) => {
                    let resource = imported_resources.entry(resource_type).or_insert_with(|| {
                        ScalaJsImportedResource::new(
                            &self.generator.context,
                            self.resolve,
                            resource_type,
                            "    ",
                        )
                    });
                    resource.add_function(
                        &self.generator.context,
                        self.resolve,
                        self,
                        func_name,
                        func,
                    );
                }
                FunctionKind::Freestanding => {}
            }
        }

        imported_resources
    }

    fn collect_exported_resources(&self) -> HashMap<TypeId, ScalaJsExportedResource> {
        let mut exported_resources = HashMap::new();

        for (_, type_id) in &self.interface.types {
            let type_def = &self.resolve.types[*type_id];
            if let TypeDefKind::Resource = &type_def.kind {
                exported_resources
                    .entry(*type_id)
                    .or_insert_with(|| ScalaJsExportedResource::new(self, *type_id));
            }
        }

        for (func_name, func) in &self.interface.functions {
            match func.kind {
                FunctionKind::Method(resource_type)
                | FunctionKind::Static(resource_type)
                | FunctionKind::Constructor(resource_type) => {
                    let resource = exported_resources
                        .entry(resource_type)
                        .or_insert_with(|| ScalaJsExportedResource::new(self, resource_type));
                    resource.add_function(func_name, func);
                }
                FunctionKind::Freestanding => {}
            }
        }

        exported_resources
    }

    fn collect_function_snippets(&self) -> Vec<String> {
        let mut functions = Vec::new();

        for (func_name, func) in &self.interface.functions {
            let func_name = self
                .generator
                .context
                .encode_name(func_name.to_lower_camel_case());

            match func.kind {
                FunctionKind::Freestanding => {
                    let args =
                        self.generator
                            .context
                            .render_args(self, self.resolve, func.params.iter());
                    let (ret, throws) =
                        self.generator
                            .context
                            .render_return_type(self, self.resolve, &func.result);

                    let mut function = String::new();
                    write_doc_comment(&mut function, "    ", &func.docs);

                    let mut postfix = match self.direction {
                        Import => " = js.native".to_string(),
                        Export => "".to_string(),
                    };

                    if let Some(throws) = throws {
                        postfix.push_str(&format!(" // throws {throws}"));
                    }

                    if self.direction == Import {
                        func_name.write_rename_attribute(&mut function, "    ");
                    }
                    uwriteln!(
                        function,
                        "    def {}({args}): {ret}{postfix}",
                        func_name.scala
                    );
                    functions.push(function);
                }
                FunctionKind::Method(_)
                | FunctionKind::Static(_)
                | FunctionKind::Constructor(_) => {}
            }
        }

        functions
    }

    pub fn finalize(self) -> ScalaJsFile {
        ScalaJsFile {
            package: self.package,
            name: self.package_object_name,
            source: self.source,
        }
    }
}
