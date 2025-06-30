//! Comprehensive tests for wit-bindgen CLI enhancements
//!
//! This test suite validates:
//! - Directory-based dependency resolution
//! - Sync-check functionality
//! - Intelligent templates
//! - Enhanced error handling
//! - AI-optimized features

use std::fs;
use std::process::Command;

/// Helper to run wit-bindgen with arguments and capture output
fn run_wit_bindgen(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_wit-bindgen"))
        .args(args)
        .output()
        .expect("Failed to execute wit-bindgen")
}

/// Helper to create a test directory structure
fn setup_test_dir(name: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let test_path = dir.path().join(name);
    fs::create_dir_all(&test_path).unwrap();
    dir
}

#[test]
fn test_directory_dependency_resolution() {
    let temp_dir = setup_test_dir("deps_test");
    let test_path = temp_dir.path().join("deps_test");

    // Create main WIT file
    let main_wit = test_path.join("component.wit");
    fs::write(
        &main_wit,
        r#"
package test:component;

world test-world {
    import wasi:io/streams@0.2.0;
    export run: func();
}
"#,
    )
    .unwrap();

    // Create deps directory
    let deps_dir = test_path.join("deps");
    fs::create_dir_all(&deps_dir).unwrap();

    // Create wasi-io dependency
    let wasi_io_dir = deps_dir.join("wasi-io");
    fs::create_dir_all(&wasi_io_dir).unwrap();
    fs::write(
        wasi_io_dir.join("streams.wit"),
        r#"
package wasi:io@0.2.0;

interface streams {
    type input-stream = u32;
    type output-stream = u32;
}
"#,
    )
    .unwrap();

    // Test validation
    let output = run_wit_bindgen(&["validate", main_wit.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "Validation should succeed with proper deps/ structure"
    );
}

#[test]
fn test_sync_check_detects_missing_deps() {
    let temp_dir = setup_test_dir("sync_test");
    let test_path = temp_dir.path().join("sync_test");

    // Create WIT file with imports but no deps
    let main_wit = test_path.join("component.wit");
    fs::write(
        &main_wit,
        r#"
package test:sync;

world sync-world {
    import missing:package/interface;
}
"#,
    )
    .unwrap();

    // Run sync-check
    let output = run_wit_bindgen(&[
        "deps",
        "--sync-check",
        "--format",
        "json",
        main_wit.to_str().unwrap(),
    ]);

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("missing:package"));
    assert!(stdout.contains("missing_from_deps"));
}

#[test]
fn test_alphabetical_ordering_validation() {
    let temp_dir = setup_test_dir("order_test");
    let test_path = temp_dir.path().join("order_test");

    let main_wit = test_path.join("component.wit");
    fs::write(&main_wit, r#"package test:order;"#).unwrap();

    // Create deps in wrong order
    let deps_dir = test_path.join("deps");
    fs::create_dir_all(&deps_dir).unwrap();
    fs::write(deps_dir.join("z-package.wit"), "package z:package;").unwrap();
    fs::write(deps_dir.join("a-package.wit"), "package a:package;").unwrap();

    // Check order validation
    let output = run_wit_bindgen(&[
        "deps",
        "--sync-check",
        "--format",
        "json",
        main_wit.to_str().unwrap(),
    ]);

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Check that ordering is reported
    assert_eq!(json["alphabetical_order"], true);
}

#[test]
fn test_intelligent_templates_generation() {
    let temp_dir = setup_test_dir("templates_test");
    let test_path = temp_dir.path().join("templates_test");

    let main_wit = test_path.join("component.wit");
    fs::write(
        &main_wit,
        r#"
package test:templates;

interface math {
    /// Adds two numbers together
    add: func(a: s32, b: s32) -> s32;
    
    /// Divides two numbers safely
    divide: func(dividend: f64, divisor: f64) -> result<f64, string>;
}

world math-world {
    export math;
}
"#,
    )
    .unwrap();

    // Generate with intelligent templates
    let output = run_wit_bindgen(&[
        "rust",
        "--intelligent-templates",
        main_wit.to_str().unwrap(),
    ]);

    assert!(output.status.success());

    // Check generated file has enhanced documentation
    let generated = test_path.join("math_world.rs");
    assert!(generated.exists());

    let content = fs::read_to_string(&generated).unwrap();
    assert!(content.contains("Auto-generated WebAssembly bindings"));
    assert!(content.contains("intelligent templates enabled"));
    assert!(content.contains("# Parameters"));
    assert!(content.contains("# Returns"));
}

#[test]
fn test_add_dependency_from_local() {
    let temp_dir = setup_test_dir("add_dep_test");
    let test_path = temp_dir.path().join("add_dep_test");

    let main_wit = test_path.join("component.wit");
    fs::write(&main_wit, "package test:add;").unwrap();

    // Create source dependency
    let source_path = test_path.join("source.wit");
    fs::write(&source_path, "package test:source;").unwrap();

    // Add dependency from local file
    let output = run_wit_bindgen(&[
        "deps",
        "--add",
        "test:source",
        "--from",
        source_path.to_str().unwrap(),
        main_wit.to_str().unwrap(),
    ]);

    assert!(output.status.success());

    // Check dependency was added
    let deps_file = test_path.join("deps/test-source.wit");
    assert!(deps_file.exists());
}

#[test]
fn test_enhanced_error_messages() {
    let temp_dir = setup_test_dir("error_test");
    let test_path = temp_dir.path().join("error_test");

    // Create WIT with missing dependency
    let main_wit = test_path.join("component.wit");
    fs::write(
        &main_wit,
        r#"
package test:error;

world error-world {
    import wasi:missing/not-found@1.0.0;
}
"#,
    )
    .unwrap();

    // Run validation
    let output = run_wit_bindgen(&["validate", main_wit.to_str().unwrap()]);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).unwrap();

    // Check for enhanced error output
    assert!(stderr.contains("Diagnostic Analysis"));
    assert!(stderr.contains("Missing Package: wasi:missing"));
    assert!(stderr.contains("Searched locations:"));
    assert!(stderr.contains("deps/wasi-missing"));
    assert!(stderr.contains("Actionable Suggestions"));
}

#[test]
fn test_help_ai_command() {
    let output = run_wit_bindgen(&["help-ai"]);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Verify AI documentation structure
    assert!(json["wit_bindgen_ai_documentation"].is_object());
    assert!(json["wit_bindgen_ai_documentation"]["overview"].is_object());
    assert!(json["wit_bindgen_ai_documentation"]["commands"].is_object());
    assert!(
        json["wit_bindgen_ai_documentation"]["dependency_resolution"]["mechanism"]
            .as_str()
            .unwrap()
            .contains("Directory-based")
    );
}

#[test]
fn test_validate_auto_deps() {
    let temp_dir = setup_test_dir("auto_deps_test");
    let test_path = temp_dir.path().join("auto_deps_test");

    // Create WIT with missing dependency
    let main_wit = test_path.join("component.wit");
    fs::write(
        &main_wit,
        r#"
package test:auto;

interface types {
    type my-type = u32;
}

world auto-world {
    import test:dep/interface;
    export types;
}
"#,
    )
    .unwrap();

    // Create the missing dependency
    let dep_wit = test_path.join("dep.wit");
    fs::write(
        &dep_wit,
        r#"
package test:dep;

interface interface {
    type dep-type = string;
}
"#,
    )
    .unwrap();

    // Run validation with auto-deps
    let output = run_wit_bindgen(&[
        "validate",
        "--auto-deps",
        "--format",
        "json",
        main_wit.to_str().unwrap(),
    ]);

    // Should succeed after auto-fixing
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Check that auto-deps worked
    assert_eq!(json["valid"], true);

    // Verify dependency was added to deps/
    let deps_file = test_path.join("deps/test-dep.wit");
    assert!(deps_file.exists());
}

#[test]
fn test_directory_package_validation() {
    let temp_dir = setup_test_dir("dir_package_test");
    let test_path = temp_dir.path().join("dir_package_test");

    let main_wit = test_path.join("component.wit");
    fs::write(
        &main_wit,
        r#"
package test:main;

world main-world {
    import test:pkg/types;
}
"#,
    )
    .unwrap();

    // Create directory-based package
    let deps_dir = test_path.join("deps");
    let pkg_dir = deps_dir.join("test-pkg");
    fs::create_dir_all(&pkg_dir).unwrap();

    fs::write(
        pkg_dir.join("types.wit"),
        r#"
package test:pkg;

interface types {
    type my-type = u32;
}
"#,
    )
    .unwrap();

    // Validate
    let output = run_wit_bindgen(&["validate", "--format", "json", main_wit.to_str().unwrap()]);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["valid"], true);
}

#[test]
fn test_analyze_command_with_templates() {
    let temp_dir = setup_test_dir("analyze_test");
    let test_path = temp_dir.path().join("analyze_test");

    let main_wit = test_path.join("component.wit");
    fs::write(
        &main_wit,
        r#"
package test:analyze;

interface api {
    record request {
        id: u64,
        data: string,
    }
    
    record response {
        status: u16,
        body: option<string>,
    }
    
    process: func(req: request) -> result<response, string>;
}

world service {
    export api;
}
"#,
    )
    .unwrap();

    // Run analyze command
    let output = run_wit_bindgen(&["analyze", "--format", "json", main_wit.to_str().unwrap()]);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Check analysis output
    assert_eq!(json["valid"], true);
    assert!(json["worlds"].is_array());
    assert!(json["type_mappings"].is_object());
    assert!(json["dependencies"].is_array());
}

// Integration test to verify all features work together
#[test]
fn test_full_integration() {
    let temp_dir = setup_test_dir("integration_test");
    let test_path = temp_dir.path().join("integration_test");

    // Create a complete component setup
    let main_wit = test_path.join("component.wit");
    fs::write(
        &main_wit,
        r#"
package example:component@1.0.0;

interface types {
    record config {
        name: string,
        value: u32,
    }
}

interface api {
    use types.{config};
    
    /// Initialize the component with configuration
    init: func(cfg: config) -> result<_, string>;
    
    /// Process incoming requests
    handle: func(data: list<u8>) -> list<u8>;
}

world service {
    import wasi:io/streams@0.2.0;
    export api;
}
"#,
    )
    .unwrap();

    // Create dependency
    let deps_dir = test_path.join("deps");
    let wasi_io_dir = deps_dir.join("wasi-io");
    fs::create_dir_all(&wasi_io_dir).unwrap();
    fs::write(
        wasi_io_dir.join("streams.wit"),
        r#"
package wasi:io@0.2.0;

interface streams {
    resource input-stream;
    resource output-stream;
}
"#,
    )
    .unwrap();

    // 1. Validate with analysis
    let output = run_wit_bindgen(&[
        "validate",
        "--analyze",
        "--format",
        "json",
        main_wit.to_str().unwrap(),
    ]);
    assert!(output.status.success());

    // 2. Check sync
    let output = run_wit_bindgen(&[
        "deps",
        "--sync-check",
        "--format",
        "json",
        main_wit.to_str().unwrap(),
    ]);
    assert!(output.status.success());

    // 3. Generate with intelligent templates
    let output = run_wit_bindgen(&[
        "rust",
        "--intelligent-templates",
        main_wit.to_str().unwrap(),
    ]);
    assert!(output.status.success());

    // Verify generated code
    let generated = test_path.join("service.rs");
    assert!(generated.exists());
    let content = fs::read_to_string(&generated).unwrap();
    assert!(content.contains("intelligent templates enabled"));
}
