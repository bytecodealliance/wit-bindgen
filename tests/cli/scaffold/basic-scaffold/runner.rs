use std::process::Command;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_basic_scaffold() {
    let temp_dir = TempDir::new().unwrap();
    let wit_path = temp_dir.path().join("test.wit");
    let output_dir = temp_dir.path().join("output");
    
    // Copy test WIT file to temp directory
    let test_wit = include_str!("test.wit");
    fs::write(&wit_path, test_wit).unwrap();
    
    // Run wit-bindgen scaffold
    let output = Command::new(env!("CARGO_BIN_EXE_wit-bindgen"))
        .args(&[
            "scaffold", 
            "--output", output_dir.to_str().unwrap(),
            wit_path.parent().unwrap().to_str().unwrap()
        ])
        .output()
        .expect("Failed to execute wit-bindgen");
    
    // Should succeed
    assert!(output.status.success(), 
           "wit-bindgen scaffold failed: {}", 
           String::from_utf8_lossy(&output.stderr));
    
    // Check that lib.rs was generated
    let lib_path = output_dir.join("lib.rs");
    assert!(lib_path.exists(), "lib.rs should be generated");
    
    // Check lib.rs content
    let lib_content = fs::read_to_string(&lib_path).unwrap();
    assert!(lib_content.contains("wit_bindgen::generate!"));
    assert!(lib_content.contains("struct Component"));
    assert!(lib_content.contains("impl exports::"));
    assert!(lib_content.contains("export!(Component)"));
    assert!(lib_content.contains("fn add("));
    assert!(lib_content.contains("fn multiply("));
    assert!(lib_content.contains("todo!()"));
    
    // Check README was generated  
    let readme_path = temp_dir.path().join("README.md");
    assert!(readme_path.exists(), "README.md should be generated");
    
    let readme_content = fs::read_to_string(&readme_path).unwrap();
    assert!(readme_content.contains("math-component"));
    assert!(readme_content.contains("Getting Started"));
    assert!(readme_content.contains("cargo build --target wasm32-wasip2"));
}

#[test]
fn test_scaffold_with_cargo() {
    let temp_dir = TempDir::new().unwrap();
    let wit_path = temp_dir.path().join("test.wit");
    let output_dir = temp_dir.path().join("src");
    
    let test_wit = include_str!("test.wit");
    fs::write(&wit_path, test_wit).unwrap();
    
    // Run with --with-cargo flag
    let output = Command::new(env!("CARGO_BIN_EXE_wit-bindgen"))
        .args(&[
            "scaffold", 
            "--with-cargo",
            "--output", output_dir.to_str().unwrap(),
            wit_path.parent().unwrap().to_str().unwrap()
        ])
        .output()
        .expect("Failed to execute wit-bindgen");
    
    assert!(output.status.success());
    
    // Check that Cargo.toml was generated
    let cargo_path = temp_dir.path().join("Cargo.toml");
    assert!(cargo_path.exists(), "Cargo.toml should be generated with --with-cargo");
    
    let cargo_content = fs::read_to_string(&cargo_path).unwrap();
    assert!(cargo_content.contains("[package]"));
    assert!(cargo_content.contains("math_component"));
    assert!(cargo_content.contains("wit-bindgen ="));
    assert!(cargo_content.contains("crate-type = [\"cdylib\"]"));
}