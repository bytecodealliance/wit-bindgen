use std::path::PathBuf;
use std::process::Command;
use wit_bindgen_core::{Files, wit_parser::Resolve};

#[test]
fn test_async_cpp_generation() {
    const WIT: &str = r#"
        package test:asyncdemo;

        interface types {
            resource descriptor {
                test: async func(arg1: string, arg2: list<s32>);
            }
            compute: async func(a: u32, b: u32) -> u32;
            fetch: async func(name: string) -> string;
        }

        world demo {
            import types;
            export run: async func() -> u32;
        }
    "#;

    let mut resolve = Resolve::default();
    let package = resolve.push_str("test.wit", WIT).unwrap();
    let world = resolve.select_world(&[package], Some("demo")).unwrap();
    let mut files = Files::default();
    let mut generator = wit_bindgen_cpp::Opts::default().build(None);

    generator.generate(&mut resolve, world, &mut files).unwrap();

    let cpp_src = String::from_utf8(files.remove("demo.cpp").unwrap()).unwrap();
    let header_src = String::from_utf8(files.remove("demo_cpp.h").unwrap()).unwrap();

    // Verify coroutine Task<T> return types
    assert!(header_src.contains("Task<uint32_t>"));
    assert!(header_src.contains("Task<wit::string>"));
    assert!(header_src.contains("Task<void>"));

    // Verify co_return in cpp source
    assert!(cpp_src.contains("co_return;"));
    assert!(cpp_src.contains("co_return result"));

    // Verify [[maybe_unused]] auto ret to prevent -Wunused-variable warnings
    assert!(cpp_src.contains("[[maybe_unused]] auto ret = "));
}

#[test]
fn test_async_resource_void_returns() {
    const WIT: &str = r#"
        package foo:bar;

        interface types {
            resource descriptor {
                test: async func(arg1: string, arg2: list<s32>);
            }
        }

        world bindings {
            import types;
        }
    "#;

    let mut resolve = Resolve::default();
    let package = resolve.push_str("test.wit", WIT).unwrap();
    let world = resolve.select_world(&[package], Some("bindings")).unwrap();
    let mut files = Files::default();
    let mut generator = wit_bindgen_cpp::Opts::default().build(None);

    generator.generate(&mut resolve, world, &mut files).unwrap();

    let cpp_src = String::from_utf8(files.remove("bindings.cpp").unwrap()).unwrap();
    let header_src = String::from_utf8(files.remove("bindings_cpp.h").unwrap()).unwrap();

    assert!(header_src.contains("Task<void>"));
    assert!(cpp_src.contains("[[maybe_unused]] auto ret = "));
    assert!(cpp_src.contains("co_return;"));
}

#[test]
fn test_async_cpp_clang_compilation() {
    let clang = match which_clangpp() {
        Some(path) => path,
        None => return,
    };

    const WIT: &str = r#"
        package test:asynccompile;

        interface api {
            compute: async func(a: u32, b: u32) -> u32;
            ping: async func();
        }

        world app {
            import api;
            export run: async func() -> u32;
        }
    "#;

    let mut resolve = Resolve::default();
    let package = resolve.push_str("test.wit", WIT).unwrap();
    let world = resolve.select_world(&[package], Some("app")).unwrap();
    let mut files = Files::default();
    let mut generator = wit_bindgen_cpp::Opts::default().build(None);

    generator.generate(&mut resolve, world, &mut files).unwrap();

    let temp_dir = std::env::temp_dir().join("wit_bindgen_cpp_async_test");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    for (name, content) in files.iter() {
        std::fs::write(temp_dir.join(name), content).unwrap();
    }

    let helper_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("helper-types");
    let test_headers_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_headers");

    let status = Command::new(&clang)
        .arg("-std=c++20")
        .arg("-I")
        .arg(&temp_dir)
        .arg("-I")
        .arg(&helper_dir)
        .arg("-I")
        .arg(&test_headers_dir)
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-Wc++-compat")
        .arg("-Wno-unused-parameter")
        .arg("-Wno-unknown-attributes")
        .arg("-fsyntax-only")
        .arg(temp_dir.join("app.cpp"))
        .status()
        .expect("failed to execute clang++");

    assert!(status.success(), "clang++ syntax check failed");
    let _ = std::fs::remove_dir_all(&temp_dir);
}

fn which_clangpp() -> Option<PathBuf> {
    for cmd in ["clang++", "wasm32-wasip2-clang++"] {
        if let Ok(output) = Command::new("which").arg(cmd).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }
    }
    None
}
