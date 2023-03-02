wit_bindgen::generate!({
    world: "world",
    path: "../../tests/runtime/wildcards",
    substitutions_path: "../../tests/runtime/wildcards/substitutions.toml",
});

struct Exports;

export_wildcards!(Exports);

impl exports::Exports for Exports {
    fn x() -> u32 {
        imports::a()
    }
    fn y() -> u32 {
        imports::b()
    }
    fn z() -> u32 {
        imports::c()
    }
}
