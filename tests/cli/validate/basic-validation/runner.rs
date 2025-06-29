use std::process::Command;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_basic_validation() {
    let temp_dir = TempDir::new().unwrap();
    let wit_path = temp_dir.path().join("test.wit");
    
    // Copy test WIT file to temp directory
    let test_wit = include_str!("test.wit");
    fs::write(&wit_path, test_wit).unwrap();
    
    // Run wit-bindgen validate
    let output = Command::new(env!("CARGO_BIN_EXE_wit-bindgen"))
        .args(&["validate", wit_path.parent().unwrap().to_str().unwrap()])
        .output()
        .expect("Failed to execute wit-bindgen");
    
    // Should succeed
    assert!(output.status.success(), 
           "wit-bindgen validate failed: {}", 
           String::from_utf8_lossy(&output.stderr));
    
    // Check expected output
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Validating WIT dependencies"));
    assert!(stderr.contains("WIT package parsed successfully"));
    assert!(stderr.contains("All validations passed"));
}

#[test] 
fn test_validation_with_show_tree() {
    let temp_dir = TempDir::new().unwrap();
    let wit_path = temp_dir.path().join("test.wit");
    
    let test_wit = include_str!("test.wit");
    fs::write(&wit_path, test_wit).unwrap();
    
    let output = Command::new(env!("CARGO_BIN_EXE_wit-bindgen"))
        .args(&["validate", "--show-tree", wit_path.parent().unwrap().to_str().unwrap()])
        .output()
        .expect("Failed to execute wit-bindgen");
    
    assert!(output.status.success());
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("World 'basic-validation' structure"));
    assert!(stderr.contains("Exports"));
    assert!(stderr.contains("export: greeting"));
}