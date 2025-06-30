// CLI Integration Tests for wit-bindgen new commands

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

// Helper function to get the wit-bindgen binary path
fn wit_bindgen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_wit-bindgen")
}

// Helper function to create a temporary WIT package
fn create_test_wit_package(temp_dir: &Path, wit_content: &str) -> std::path::PathBuf {
    let wit_file = temp_dir.join("test.wit");
    fs::write(&wit_file, wit_content).unwrap();
    temp_dir.to_path_buf()
}

mod validate_tests {
    use super::*;

    #[test]
    fn basic_validation_success() {
        let temp_dir = TempDir::new().unwrap();
        let wit_content = r#"
package test:basic@1.0.0;

interface greeting {
    hello: func(name: string) -> string;
}

world basic {
    export greeting;
}
"#;
        let wit_dir = create_test_wit_package(temp_dir.path(), wit_content);

        let output = Command::new(wit_bindgen_bin())
            .args(&["validate", wit_dir.to_str().unwrap()])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "Validation should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("WIT package parsed successfully"));
        assert!(stderr.contains("All validations passed"));
    }

    #[test]
    fn validation_with_show_tree() {
        let temp_dir = TempDir::new().unwrap();
        let wit_content = r#"
package test:tree@1.0.0;

interface math {
    add: func(a: u32, b: u32) -> u32;
}

world calculator {
    export math;
    import wasi:io/streams@0.2.0;
}
"#;
        let wit_dir = create_test_wit_package(temp_dir.path(), wit_content);

        let output = Command::new(wit_bindgen_bin())
            .args(&["validate", "--show-tree", wit_dir.to_str().unwrap()])
            .output()
            .unwrap();

        assert!(output.status.success());

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("World 'calculator' structure"));
        assert!(stderr.contains("export: math"));
    }

    #[test]
    fn validation_invalid_syntax() {
        let temp_dir = TempDir::new().unwrap();
        let wit_content = r#"
package test:invalid@1.0.0;

interface broken {
    // Missing semicolon - invalid syntax
    invalid-func: func() -> string
}
"#;
        let wit_dir = create_test_wit_package(temp_dir.path(), wit_content);

        let output = Command::new(wit_bindgen_bin())
            .args(&["validate", wit_dir.to_str().unwrap()])
            .output()
            .unwrap();

        assert!(!output.status.success(), "Should fail on invalid syntax");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("error: WIT validation failed"));
    }

    #[test]
    fn validation_nonexistent_directory() {
        let output = Command::new(wit_bindgen_bin())
            .args(&["validate", "/nonexistent/directory"])
            .output()
            .unwrap();

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("error: WIT validation failed"));
    }
}

mod scaffold_tests {
    use super::*;

    #[test]
    fn basic_scaffolding() {
        let temp_dir = TempDir::new().unwrap();
        let wit_content = r#"
package test:scaffold@1.0.0;

interface calculator {
    add: func(a: u32, b: u32) -> u32;
    subtract: func(a: u32, b: u32) -> u32;
}

world math {
    export calculator;
}
"#;
        let wit_dir = create_test_wit_package(temp_dir.path(), wit_content);
        let output_dir = temp_dir.path().join("output");

        let output = Command::new(wit_bindgen_bin())
            .args(&[
                "scaffold",
                "--output",
                output_dir.to_str().unwrap(),
                wit_dir.to_str().unwrap(),
            ])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "Scaffolding should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Check generated lib.rs
        let lib_path = output_dir.join("lib.rs");
        assert!(lib_path.exists());

        let lib_content = fs::read_to_string(&lib_path).unwrap();
        assert!(lib_content.contains("wit_bindgen::generate!"));
        assert!(lib_content.contains("world: \"math\""));
        assert!(lib_content.contains("struct Component"));
        assert!(lib_content.contains("fn add("));
        assert!(lib_content.contains("fn subtract("));
        assert!(lib_content.contains("todo!()"));

        // Check generated README
        let readme_path = temp_dir.path().join("README.md");
        assert!(readme_path.exists());

        let readme_content = fs::read_to_string(&readme_path).unwrap();
        assert!(readme_content.contains("math"));
        assert!(readme_content.contains("Getting Started"));
    }

    #[test]
    fn scaffolding_with_cargo() {
        let temp_dir = TempDir::new().unwrap();
        let wit_content = r#"
package test:cargo@1.0.0;

interface simple {
    process: func() -> string;
}

world processor {
    export simple;
}
"#;
        let wit_dir = create_test_wit_package(temp_dir.path(), wit_content);
        let output_dir = temp_dir.path().join("src");

        let output = Command::new(wit_bindgen_bin())
            .args(&[
                "scaffold",
                "--with-cargo",
                "--name",
                "my_processor",
                "--output",
                output_dir.to_str().unwrap(),
                wit_dir.to_str().unwrap(),
            ])
            .output()
            .unwrap();

        assert!(output.status.success());

        // Check Cargo.toml was generated
        let cargo_path = temp_dir.path().join("Cargo.toml");
        assert!(cargo_path.exists());

        let cargo_content = fs::read_to_string(&cargo_path).unwrap();
        assert!(cargo_content.contains("name = \"my_processor\""));
        assert!(cargo_content.contains("wit-bindgen ="));
        assert!(cargo_content.contains("crate-type = [\"cdylib\"]"));

        // Check lib.rs in src directory
        let lib_path = output_dir.join("lib.rs");
        assert!(lib_path.exists());
    }

    #[test]
    fn scaffolding_invalid_wit() {
        let temp_dir = TempDir::new().unwrap();
        let wit_content = r#"
invalid wit content that cannot be parsed
"#;
        let wit_dir = create_test_wit_package(temp_dir.path(), wit_content);
        let output_dir = temp_dir.path().join("output");

        let output = Command::new(wit_bindgen_bin())
            .args(&[
                "scaffold",
                "--output",
                output_dir.to_str().unwrap(),
                wit_dir.to_str().unwrap(),
            ])
            .output()
            .unwrap();

        assert!(!output.status.success(), "Should fail on invalid WIT");
    }
}

mod interactive_tests {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};

    #[test]
    fn interactive_help_display() {
        let temp_dir = TempDir::new().unwrap();
        let wit_content = r#"
package test:interactive@1.0.0;

interface demo {
    run: func() -> string;
}

world interactive {
    export demo;
}
"#;
        let wit_dir = create_test_wit_package(temp_dir.path(), wit_content);

        // Test that interactive mode starts and shows help
        let mut child = Command::new(wit_bindgen_bin())
            .args(&["interactive", wit_dir.to_str().unwrap()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        // Send "4" to exit immediately
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(b"4\n").unwrap();
        }

        let output = child.wait_with_output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should show interactive mode startup
        assert!(stderr.contains("Welcome to wit-bindgen Interactive Mode"));
        assert!(stderr.contains("Step 1: Validating WIT dependencies"));
        assert!(stderr.contains("Step 3: Choose your next action"));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn end_to_end_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let wit_content = r#"
package test:e2e@1.0.0;

interface string-utils {
    reverse: func(input: string) -> string;
    length: func(input: string) -> u32;
}

world text-processor {
    export string-utils;
}
"#;
        let wit_dir = create_test_wit_package(temp_dir.path(), wit_content);

        // Step 1: Validate the WIT
        let validate_output = Command::new(wit_bindgen_bin())
            .args(&["validate", wit_dir.to_str().unwrap()])
            .output()
            .unwrap();

        assert!(validate_output.status.success());

        // Step 2: Generate scaffolding
        let output_dir = temp_dir.path().join("generated");
        let scaffold_output = Command::new(wit_bindgen_bin())
            .args(&[
                "scaffold",
                "--with-cargo",
                "--name",
                "text_processor",
                "--output",
                output_dir.to_str().unwrap(),
                wit_dir.to_str().unwrap(),
            ])
            .output()
            .unwrap();

        assert!(scaffold_output.status.success());

        // Step 3: Verify all expected files exist
        assert!(temp_dir.path().join("Cargo.toml").exists());
        assert!(output_dir.join("lib.rs").exists());
        assert!(temp_dir.path().join("README.md").exists());

        // Step 4: Check that the generated code has proper structure
        let lib_content = fs::read_to_string(output_dir.join("lib.rs")).unwrap();
        assert!(lib_content.contains("fn reverse("));
        assert!(lib_content.contains("fn length("));
        assert!(lib_content.contains("input: String"));
        assert!(lib_content.contains("-> u32"));
    }
}
