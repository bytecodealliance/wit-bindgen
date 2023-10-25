// TODO: Implement tests similar to the other generators.
// This requires that we have any dependencies either included here or published to NuGet or similar.
use std::path::Path;
use wit_component::StringEncoding;

macro_rules! codegen_test {
    ($id:ident $name:tt $test:tt) => {
        #[test]
        fn $id() {
            test_helpers::run_world_codegen_test(
                "guest-csharp",
                $test.as_ref(),
                |resolve, world, files| {
                    if [
                        "conventions",
                        "flags",
                        "guest-name",
                        "import-and-export-resource",
                        "import-and-export-resource-alias",
                        "import-func",
                        "integers",
                        "issue544",
                        "issue551",
                        "issue569",
                        "issue573",
                        "issue607",
                        "issue668",
                        "just-export",
                        "keywords",
                        "lift-lower-foreign",
                        "lists",
                        "many-arguments",
                        "multi-return",
                        "option-result",
                        "records",
                        "rename-interface",
                        "resource-alias",
                        "resource-borrow-in-record",
                        "resource-borrow-in-record-export",
                        "resource-local-alias",
                        "resource-local-alias-borrow",
                        "resource-local-alias-borrow-import",
                        "resource-own-in-other-interface",
                        "resources",
                        "resources-in-aggregates",
                        "resources-with-lists",
                        "result-empty",
                        "ret-areas",
                        "return-resource-from-export",
                        "same-names5",
                        "simple-functions",
                        "simple-http",
                        "simple-lists",
                        "small-anonymous",
                        "unused-import",
                        "use-across-interfaces",
                        "variants",
                        "worlds-with-types",
                        "zero-size-tuple",
                    ]
                    .contains(&$name)
                    {
                        return;
                    }
                    wit_bindgen_csharp::Opts {
                        generate_stub: true,
                        string_encoding: StringEncoding::UTF8,
                    }
                    .build()
                    .generate(resolve, world, files)
                    .unwrap()
                },
                verify,
            )
        }
    };
}
test_helpers::codegen_tests!();

fn verify(_dir: &Path, _name: &str) {
    // TODO?
}
