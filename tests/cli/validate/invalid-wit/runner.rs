use std::process::Command;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_invalid_wit_validation() {
    let temp_dir = TempDir::new().unwrap();
    let wit_path = temp_dir.path().join("test.wit");
    
    // Copy invalid WIT file to temp directory
    let test_wit = include_str!("test.wit");
    fs::write(&wit_path, test_wit).unwrap();
    
    // Run wit-bindgen validate - should fail
    let output = Command::new(env!("CARGO_BIN_EXE_wit-bindgen"))
        .args(&["validate", wit_path.parent().unwrap().to_str().unwrap()])
        .output()
        .expect("Failed to execute wit-bindgen");
    
    // Should fail
    assert!(!output.status.success(), 
           "wit-bindgen validate should have failed on invalid WIT");
    
    // Check that we get helpful error messages
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error: WIT validation failed"));
    assert!(stderr.contains("Suggestions") || stderr.contains("General troubleshooting"));
}

#[test]
fn test_missing_directory() {
    // Run wit-bindgen validate on non-existent directory
    let output = Command::new(env!("CARGO_BIN_EXE_wit-bindgen"))
        .args(&["validate", "/non/existent/path"])
        .output()
        .expect("Failed to execute wit-bindgen");
    
    // Should fail
    assert!(!output.status.success());
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error: WIT validation failed"));
}