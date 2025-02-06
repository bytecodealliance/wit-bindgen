use crate::context::{package_name_to_segments, write_doc_comment, ScalaJsContext};
use crate::resource::ScalaJsImportedResource;
use crate::ScalaJsFile;
use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use std::collections::HashMap;
use std::fmt::Write;
use wit_bindgen_core::uwriteln;
use wit_bindgen_core::wit_parser::{
    Function, FunctionKind, Resolve, TypeDefKind, TypeId, World, WorldId,
};
use wit_bindgen_core::Direction::{Export, Import};

pub struct ScalaJsWorld {
    world_id: WorldId,
    header: String,
    types: String,
    global_imports: String,
    imported_resources: HashMap<TypeId, ScalaJsImportedResource>,
    export_header: String,
    global_exports: String,
}

impl ScalaJsWorld {
    pub fn new(
        context: &ScalaJsContext,
        resolve: &Resolve,
        world_id: WorldId,
        world: &World,
    ) -> Self {
        let name = world.name.clone().to_snake_case();

        let package_name = resolve.packages[world.package.expect("missing package for world")]
            .name
            .clone();

        let package = package_name_to_segments(
            &context.opts,
            &package_name,
            &Import,
            &context.keywords,
            false,
        );

        let mut header = String::new();
        uwriteln!(header, "package {}", package.join("."));

        uwriteln!(header, "");
        uwriteln!(header, "import scala.scalajs.js");
        uwriteln!(header, "import scala.scalajs.js.annotation._");
        uwriteln!(header, "import {}wit._", context.opts.base_package_prefix());
        uwriteln!(header, "");
        uwriteln!(header, "package object {} {{", name);

        let export_package = package_name_to_segments(
            &context.opts,
            &package_name,
            &Export,
            &context.keywords,
            false,
        );

        let encoded_world_name = context.encode_name(world.name.clone().to_pascal_case());

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
        uwriteln!(export_header, "package object {} {{", name);
        uwriteln!(export_header, "  trait {} {{", encoded_world_name.scala);

        Self {
            world_id,
            header,
            types: String::new(),
            global_imports: String::new(),
            imported_resources: HashMap::new(),
            export_header,
            global_exports: String::new(),
        }
    }

    pub fn add_imported_function(
        &mut self,
        context: &ScalaJsContext,
        resolve: &Resolve,
        func_name: &str,
        func: &Function,
    ) {
        match func.kind {
            FunctionKind::Freestanding => {
                uwriteln!(self.global_imports, "  @js.native");
                uwriteln!(
                    self.global_imports,
                    "  @JSImport(\"{}\", JSImport.Default)",
                    func_name
                );
                let encoded_name = context.encode_name(func_name.to_lower_camel_case());
                let args = context.render_args(context, resolve, func.params.iter());
                let ret = context.render_return_type(context, resolve, &func.results);

                write_doc_comment(&mut self.global_imports, "  ", &func.docs);
                encoded_name.write_rename_attribute(&mut self.global_imports, "  ");
                uwriteln!(
                    self.global_imports,
                    "  def {}({args}): {ret} = js.native",
                    encoded_name.scala
                );
            }
            FunctionKind::Method(resource_type)
            | FunctionKind::Static(resource_type)
            | FunctionKind::Constructor(resource_type) => {
                let resource = self
                    .imported_resources
                    .entry(resource_type)
                    .or_insert_with(|| {
                        ScalaJsImportedResource::new(context, resolve, resource_type, "  ")
                    });
                resource.add_function(context, resolve, context, func_name, func);
            }
        }
    }

    pub fn add_type(
        &mut self,
        context: &ScalaJsContext,
        resolve: &Resolve,
        name: &str,
        id: &TypeId,
    ) {
        let typ = &resolve.types[*id];
        if let Some(typ) = context.render_typedef(context, resolve, name, typ) {
            uwriteln!(self.types, "{}", typ);
            uwriteln!(self.types, "");
        }

        if let TypeDefKind::Resource = &typ.kind {
            self.imported_resources
                .entry(*id)
                .or_insert_with(|| ScalaJsImportedResource::new(context, resolve, *id, "  "));
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

                write_doc_comment(&mut self.global_exports, "  ", &func.docs);
                uwriteln!(
                    self.global_exports,
                    "    def {}({args}): {ret}",
                    encoded_name.scala
                );
            }
            FunctionKind::Method(_resource_type)
            | FunctionKind::Static(_resource_type)
            | FunctionKind::Constructor(_resource_type) => {
                panic!("Exported inline resource functions are not supported")
            }
        }
    }

    pub fn finalize(mut self, context: &ScalaJsContext, resolve: &Resolve) -> Vec<ScalaJsFile> {
        let mut source = String::new();
        uwriteln!(source, "{}", self.header);
        uwriteln!(source, "{}", self.types);
        uwriteln!(source, "{}", self.global_imports);

        for (_, mut imported_resource) in self.imported_resources.drain() {
            imported_resource.annotate(&format!(
                "@JSImport(\"{}\", JSImport.Default)",
                imported_resource.name.js
            ));
            uwriteln!(source, "{}", imported_resource.finalize());
        }

        uwriteln!(source, "}}");

        let mut export_source = String::new();
        uwriteln!(export_source, "{}", self.export_header);
        uwriteln!(export_source, "{}", self.global_exports);
        uwriteln!(export_source, "  }}");
        uwriteln!(export_source, "}}");

        let world = &resolve.worlds[self.world_id];
        let package_name = resolve.packages[world.package.expect("missing package for world")]
            .name
            .clone();

        let package = package_name_to_segments(
            &context.opts,
            &package_name,
            &Import,
            &context.keywords,
            false,
        );
        let export_package = package_name_to_segments(
            &context.opts,
            &package_name,
            &Export,
            &context.keywords,
            false,
        );

        vec![
            ScalaJsFile {
                package,
                name: world.name.to_snake_case(),
                source,
            },
            ScalaJsFile {
                package: export_package,
                name: world.name.to_snake_case(),
                source: export_source,
            },
        ]
    }
}
