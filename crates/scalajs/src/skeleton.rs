use crate::context::{
    package_name_to_segments, write_doc_comment, EncodedName, ScalaJsContext, ScalaJsFile,
};
use crate::ScalaJs;
use heck::{ToKebabCase, ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use std::collections::HashMap;
use std::fmt::Write;
use wit_bindgen_core::wit_parser::{
    Function, FunctionKind, Interface, InterfaceId, Resolve, TypeDefKind, TypeId, World, WorldId,
};
use wit_bindgen_core::Direction::Export;
use wit_bindgen_core::{uwrite, uwriteln};

pub struct ScalaJsInterfaceSkeleton<'a> {
    wit_name: String,
    name: String,
    source: String,
    package: Vec<String>,
    binding_package: Vec<String>,
    pub resolve: &'a Resolve,
    interface: &'a Interface,
    pub interface_id: InterfaceId,
    pub generator: &'a mut ScalaJs,
}

impl<'a> ScalaJsInterfaceSkeleton<'a> {
    pub fn new(
        wit_name: String,
        resolve: &'a Resolve,
        interface_id: InterfaceId,
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
            &Export,
            &generator.context.keywords,
            true,
        );

        let binding_package = package_name_to_segments(
            &generator.context.opts,
            &package_name,
            &Export,
            &generator.context.keywords,
            false,
        );

        Self {
            wit_name,
            name,
            source: "".to_string(),
            package,
            binding_package,
            resolve,
            interface,
            interface_id,
            generator,
        }
    }

    pub fn generate(&mut self) {
        let mut source = String::new();
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

        let base_trait_name = format!(
            "{}.{}.{}",
            self.binding_package.join("."),
            self.generator
                .context
                .keywords
                .escape(self.name.to_snake_case()),
            self.name
        );

        let encoded_name = self.generator.context.encode_name(&self.name.to_kebab_case());

        uwriteln!(source, "@JSExportTopLevel(\"{}\")", encoded_name.js);
        uwriteln!(
            source,
            "object {} extends {} {{",
            self.name,
            base_trait_name
        );

        for function in self.collect_function_snippets() {
            uwriteln!(source, "{}", function);
        }

        uwriteln!(source, "}}");

        let exported_resources = self.collect_exported_resources();

        uwriteln!(source, "");
        for (_, resource) in exported_resources {
            uwriteln!(source, "{}", resource.finalize());
            uwriteln!(source, "");
        }

        self.source = source;
    }

    pub fn finalize(self) -> ScalaJsFile {
        ScalaJsFile {
            package: self.package,
            name: self.name,
            source: self.source,
        }
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
                    let ret = self.generator.context.render_return_type(
                        self,
                        self.resolve,
                        &func.results,
                    );

                    let mut function = String::new();

                    func_name.write_export_attribute(&mut function, "    ");
                    uwriteln!(
                        function,
                        "    override def {}({args}): {ret} = {{",
                        func_name.scala
                    );
                    uwriteln!(function, "        ???");
                    uwriteln!(function, "    }}");
                    functions.push(function);
                }
                FunctionKind::Method(_)
                | FunctionKind::Static(_)
                | FunctionKind::Constructor(_) => {}
            }
        }

        functions
    }

    fn collect_exported_resources(&self) -> HashMap<TypeId, ScalaJsExportedResourceSkeleton> {
        let mut exported_resources = HashMap::new();

        for (_, type_id) in &self.interface.types {
            let type_def = &self.resolve.types[*type_id];
            if let TypeDefKind::Resource = &type_def.kind {
                exported_resources
                    .entry(*type_id)
                    .or_insert_with(|| ScalaJsExportedResourceSkeleton::new(self, *type_id));
            }
        }

        for (func_name, func) in &self.interface.functions {
            match func.kind {
                FunctionKind::Method(resource_type)
                | FunctionKind::Static(resource_type)
                | FunctionKind::Constructor(resource_type) => {
                    let resource = exported_resources.entry(resource_type).or_insert_with(|| {
                        ScalaJsExportedResourceSkeleton::new(self, resource_type)
                    });
                    resource.add_function(func_name, func);
                }
                FunctionKind::Freestanding => {}
            }
        }

        exported_resources
    }
}

pub struct ScalaJsWorldSkeleton {
    world_id: WorldId,
    export_header: String,
    global_exports: String,
    export_package: Vec<String>,
}

impl ScalaJsWorldSkeleton {
    pub fn new(
        context: &ScalaJsContext,
        resolve: &Resolve,
        world_id: WorldId,
        world: &World,
    ) -> Self {
        let package_name = resolve.packages[world.package.expect("missing package for world")]
            .name
            .clone();

        let export_package = package_name_to_segments(
            &context.opts,
            &package_name,
            &Export,
            &context.keywords,
            true,
        );

        let binding_package = package_name_to_segments(
            &context.opts,
            &package_name,
            &Export,
            &context.keywords,
            false,
        );

        let encoded_name = context.encode_name(world.name.to_pascal_case());

        let base_trait_name = format!(
            "{}.{}.{}",
            binding_package.join("."),
            context.keywords.escape(world.name.to_snake_case()),
            encoded_name.scala
        );

        let mut export_header = String::new();
        uwriteln!(export_header, "package {}", export_package.join("."));
        uwriteln!(export_header, "");
        uwriteln!(export_header, "import scala.scalajs.js");
        uwriteln!(export_header, "import scala.scalajs.js.annotation._");
        uwriteln!(
            export_header,
            "import {}wit._",
            context.opts.base_package_prefix()
        );
        uwriteln!(export_header, "");
        uwriteln!(
            export_header,
            "object {} extends {base_trait_name} {{",
            encoded_name.scala
        );

        Self {
            world_id,
            export_header,
            global_exports: String::new(),
            export_package,
        }
    }

    pub fn add_exported_function(
        &mut self,
        context: &ScalaJsContext,
        resolve: &Resolve,
        func_name: &str,
        func: &Function,
    ) {
        match func.kind {
            FunctionKind::Freestanding => {
                let encoded_name = context.encode_name(func_name.to_lower_camel_case());
                let args = context.render_args(context, resolve, func.params.iter());
                let ret = context.render_return_type(context, resolve, &func.results);

                uwriteln!(
                    self.global_exports,
                    "  def {}({args}): {ret} = {{",
                    encoded_name.scala
                );
                uwriteln!(self.global_exports, "      ???");
                uwriteln!(self.global_exports, " }}");
            }
            FunctionKind::Method(_resource_type)
            | FunctionKind::Static(_resource_type)
            | FunctionKind::Constructor(_resource_type) => {
                panic!("Exported inline resource functions are not supported")
            }
        }
    }

    pub fn finalize(self, resolve: &Resolve) -> ScalaJsFile {
        let mut export_source = String::new();
        uwriteln!(export_source, "{}", self.export_header);
        uwriteln!(export_source, "{}", self.global_exports);
        uwriteln!(export_source, "}}");

        let world = &resolve.worlds[self.world_id];

        ScalaJsFile {
            package: self.export_package,
            name: world.name.to_snake_case(),
            source: export_source,
        }
    }
}

pub struct ScalaJsExportedResourceSkeleton<'a> {
    owner: &'a ScalaJsInterfaceSkeleton<'a>,
    _resource_id: TypeId,
    resource_name: String,
    encoded_resource_name: EncodedName,
    class_header: String,
    base_class_header: String,
    class_source: String,
    static_methods: String,
    constructor_args: String,
    base_constructor_args: String,
    base_static_trait_name: String,
}

impl<'a> ScalaJsExportedResourceSkeleton<'a> {
    pub fn new(owner: &'a ScalaJsInterfaceSkeleton<'a>, resource_id: TypeId) -> Self {
        let resource = &owner.resolve.types[resource_id];
        let resource_name = resource
            .name
            .clone()
            .expect("Anonymous resources not supported");
        let encoded_resource_name = owner
            .generator
            .context
            .encode_name(resource_name.to_pascal_case());

        let base_class_name = format!(
            "{}.{}.{}",
            owner.binding_package.join("."),
            owner
                .generator
                .context
                .keywords
                .escape(owner.name.to_snake_case()),
            encoded_resource_name.scala
        );

        let base_static_trait_name = format!(
            "{}.{}.{}",
            owner.binding_package.join("."),
            owner
                .generator
                .context
                .keywords
                .escape(owner.name.to_snake_case()),
            format!("{}Static", encoded_resource_name.scala)
        );

        let mut class_header = String::new();
        write_doc_comment(&mut class_header, "    ", &resource.docs);

        uwriteln!(
            class_header,
            "@JSExportTopLevel(\"{}\")",
            encoded_resource_name.js
        );
        uwrite!(class_header, "class {}(", encoded_resource_name.scala);

        let mut base_class_header = String::new();
        uwrite!(base_class_header, ") extends {base_class_name}(");

        let mut class_source = String::new();
        uwriteln!(class_source, ") {{");

        Self {
            owner,
            _resource_id: resource_id,
            resource_name,
            encoded_resource_name,
            class_header,
            base_class_header,
            class_source,
            static_methods: String::new(),
            constructor_args: String::new(),
            base_constructor_args: String::new(),
            base_static_trait_name,
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

                encoded_func_name.write_rename_attribute(&mut self.class_source, "  ");
                uwriteln!(
                    self.class_source,
                    "  override def {}({args}): {ret} = {{",
                    encoded_func_name.scala
                );
                uwriteln!(self.class_source, "    ???");
                uwriteln!(self.class_source, "  }}");
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
                encoded_func_name.write_static_export_attribute(&mut self.static_methods, "  ");
                uwriteln!(
                    self.static_methods,
                    "  override def {}({args}): {ret} = {{",
                    encoded_func_name.scala
                );
                uwriteln!(self.static_methods, "    ???");
                uwriteln!(self.static_methods, "  }}");
            }
            FunctionKind::Constructor(_) => {
                let args = self.owner.generator.context.render_args(
                    self.owner,
                    self.owner.resolve,
                    func.params.iter(),
                );
                self.constructor_args = args;

                let mut param_names = Vec::new();
                for (param_name, _) in func.params.iter() {
                    let param_name = self
                        .owner
                        .generator
                        .context
                        .encode_name(param_name.to_lower_camel_case());
                    param_names.push(param_name.scala);
                }
                self.base_constructor_args = param_names.join(", ");
            }
        }
    }

    pub fn finalize(self) -> String {
        let mut class_source = self.class_header;
        uwrite!(class_source, "{}", self.constructor_args);
        uwrite!(class_source, "{}", self.base_class_header);
        uwrite!(class_source, "{}", self.base_constructor_args);
        uwriteln!(class_source, "{}", self.class_source);
        uwriteln!(class_source, "}}");
        uwriteln!(class_source, "");

        uwriteln!(
            class_source,
            "object {} extends {} {{",
            self.encoded_resource_name.scala,
            self.base_static_trait_name
        );
        uwriteln!(class_source, "{}", self.static_methods);
        uwriteln!(class_source, "}}");
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
