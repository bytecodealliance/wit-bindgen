use wit_bindgen_core::WorldGenerator;
use wit_component::StringEncoding;

mod csharp_ident;
mod csproj;
mod function;
mod interface;
mod world_generator;

pub use csproj::CSProject;

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    #[cfg_attr(feature = "clap", arg(long, default_value_t = StringEncoding::default()))]
    pub string_encoding: StringEncoding,

    /// Whether or not to generate a stub class for exported functions
    #[cfg_attr(feature = "clap", arg(long))]
    pub generate_stub: bool,

    // TODO: This should only temporarily needed until mono and native aot aligns.
    #[cfg_attr(feature = "clap", arg(short, long, value_enum))]
    pub runtime: CSharpRuntime,

    /// Use the `internal` access modifier by default instead of `public`
    #[cfg_attr(feature = "clap", arg(long))]
    pub internal: bool,

    /// Skip generating `cabi_realloc`, `WasmImportLinkageAttribute`, and component type files
    #[cfg_attr(feature = "clap", arg(long))]
    pub skip_support_files: bool,

    /// Generate code for WIT `Result` types instead of exceptions
    #[cfg_attr(feature = "clap", arg(long))]
    pub with_wit_results: bool,
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        Box::new(world_generator::CSharp {
            opts: self.clone(),
            ..world_generator::CSharp::default()
        })
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum CSharpRuntime {
    #[default]
    NativeAOT,
    Mono,
}

#[cfg(test)]
mod tests {
    use crate::csharp_ident::ToCSharpIdent;
    use crate::interface::{
        resource_func_name, resource_handle_property_name, resource_thandle_struct_name,
        resource_wasm_interop_class_name, variant_accessor_name, variant_new_func_name,
        variant_tag_const_name, variant_tag_field_name, variant_tags_class_name,
    };

    #[test]
    fn test_csharp_ident_keywords() {
        assert_eq!("delegate".to_csharp_ident(), "@delegate");
        assert_eq!("DELEGATE".to_csharp_ident(), "@delegate");
        assert_eq!("Delegate".to_csharp_ident(), "@delegate");
        assert_eq!("class".to_csharp_ident(), "@class");
        assert_eq!("CLASS".to_csharp_ident(), "@class");
        assert_eq!("normal-name".to_csharp_ident(), "normalName");
    }

    #[test]
    fn test_variant_collision_helpers() {
        assert_eq!(variant_tag_field_name("Foo"), "Tag");
        assert_eq!(variant_tag_field_name("Tag"), "Tag_");

        assert_eq!(variant_tags_class_name("Foo"), "Tags");
        assert_eq!(variant_tags_class_name("Tags"), "Tags_");

        assert_eq!(variant_tag_const_name("Tags", "Foo"), "Foo");
        assert_eq!(variant_tag_const_name("Tags", "Tags"), "Tags_");
        assert_eq!(variant_tag_const_name("Tags_", "Tags"), "Tags");

        assert_eq!(variant_accessor_name("Foo", "bar"), "AsBar");
        assert_eq!(variant_accessor_name("AsBar", "bar"), "AsBar_");

        let accessors = vec!["AsTag".to_string(), "AsFoo".to_string()];
        assert_eq!(
            variant_new_func_name("Ingredient", "Tag", &accessors),
            "Tag_"
        );
        assert_eq!(
            variant_new_func_name("Ingredient", "Tags", &accessors),
            "Tags_"
        );
        assert_eq!(
            variant_new_func_name("Ingredient", "Ingredient", &accessors),
            "Ingredient_"
        );
        assert_eq!(
            variant_new_func_name("Ingredient", "AsFoo", &accessors),
            "AsFoo_"
        );
        assert_eq!(variant_new_func_name("Tag", "Tag", &accessors), "Tag__");
        assert_eq!(variant_new_func_name("Tags", "Tags", &accessors), "Tags__");
    }

    #[test]
    fn test_resource_collision_helpers() {
        assert_eq!(resource_handle_property_name("Foo"), "Handle");
        assert_eq!(resource_handle_property_name("Handle"), "Handle_");

        assert_eq!(resource_thandle_struct_name("Foo"), "THandle");
        assert_eq!(resource_thandle_struct_name("THandle"), "THandle_");

        assert_eq!(resource_wasm_interop_class_name("Foo"), "WasmInterop");
        assert_eq!(
            resource_wasm_interop_class_name("WasmInterop"),
            "WasmInterop_"
        );

        assert_eq!(resource_func_name("Foo", "Handle"), "Handle_");
        assert_eq!(resource_func_name("Foo", "THandle"), "THandle_");
        assert_eq!(resource_func_name("Foo", "WasmInterop"), "WasmInterop_");
        assert_eq!(resource_func_name("Foo", "Dispose"), "Dispose_");
        assert_eq!(resource_func_name("Foo", "Foo"), "Foo_");
        assert_eq!(resource_func_name("Dispose", "Dispose"), "Dispose__");
    }
}
