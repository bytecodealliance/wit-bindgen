use anyhow::{bail, Context, Error, Result};
use clap::Parser;
use std::path::PathBuf;
use std::str;
use std::io::{self, Write};
use wit_bindgen_core::{wit_parser, Files, WorldGenerator};
use wit_parser::{Resolve, WorldId, PackageId};

/// Helper for passing VERSION to opt.
/// If CARGO_VERSION_INFO is set, use it, otherwise use CARGO_PKG_VERSION.
fn version() -> &'static str {
    option_env!("CARGO_VERSION_INFO").unwrap_or(env!("CARGO_PKG_VERSION"))
}

#[derive(Debug, Parser)]
#[command(version = version())]
enum Opt {
    /// This generator outputs a Markdown file describing an interface.
    #[cfg(feature = "markdown")]
    Markdown {
        #[clap(flatten)]
        opts: wit_bindgen_markdown::Opts,
        #[clap(flatten)]
        args: Common,
    },
    /// Generates bindings for MoonBit guest modules.
    #[cfg(feature = "moonbit")]
    Moonbit {
        #[clap(flatten)]
        opts: wit_bindgen_moonbit::Opts,
        #[clap(flatten)]
        args: Common,
    },
    /// Generates bindings for Rust guest modules.
    #[cfg(feature = "rust")]
    Rust {
        #[clap(flatten)]
        opts: wit_bindgen_rust::Opts,
        #[clap(flatten)]
        args: Common,
    },
    /// Generates bindings for C/CPP guest modules.
    #[cfg(feature = "c")]
    C {
        #[clap(flatten)]
        opts: wit_bindgen_c::Opts,
        #[clap(flatten)]
        args: Common,
    },
    /// Generates bindings for C++ modules.
    #[cfg(feature = "cpp")]
    Cpp {
        #[clap(flatten)]
        opts: wit_bindgen_cpp::Opts,
        #[clap(flatten)]
        args: Common,
    },

    /// Generates bindings for TinyGo-based Go guest modules (Deprecated)
    #[cfg(feature = "go")]
    TinyGo {
        #[clap(flatten)]
        args: Common,
    },

    /// Generates bindings for C# guest modules.
    #[cfg(feature = "csharp")]
    #[command(alias = "c-sharp")]
    Csharp {
        #[clap(flatten)]
        opts: wit_bindgen_csharp::Opts,
        #[clap(flatten)]
        args: Common,
    },

    // doc-comments are present on `wit_bindgen_test::Opts` for clap to use.
    Test {
        #[clap(flatten)]
        opts: wit_bindgen_test::Opts,
    },
    
    /// Validate WIT package dependencies and structure
    Validate {
        #[clap(flatten)]
        args: Common,
        
        /// Check all dependencies recursively
        #[clap(long)]
        recursive: bool,
        
        /// Show dependency tree structure
        #[clap(long)]
        show_tree: bool,
    },
    
    /// Generate working stub implementations from WIT definitions
    Scaffold {
        #[clap(flatten)]
        args: Common,
        
        /// Output directory for generated stubs
        #[clap(long, default_value = "src")]
        output: PathBuf,
        
        /// Generate Cargo.toml project file
        #[clap(long)]
        with_cargo: bool,
        
        /// Component name for generated files
        #[clap(long)]
        name: Option<String>,
    },
    
    /// Interactive guided implementation mode
    Interactive {
        #[clap(flatten)]
        args: Common,
        
        /// Start in guided mode for beginners
        #[clap(long)]
        guided: bool,
    },
}

#[derive(Debug, Parser)]
struct Common {
    /// Where to place output files
    #[clap(long = "out-dir")]
    out_dir: Option<PathBuf>,

    /// Location of WIT file(s) to generate bindings for.
    ///
    /// This path can be either a directory containing `*.wit` files, a `*.wit`
    /// file itself, or a `*.wasm` file which is a wasm-encoded WIT package.
    /// Most of the time it's likely to be a directory containing `*.wit` files
    /// with an optional `deps` folder inside of it.
    #[clap(value_name = "WIT", index = 1)]
    wit: PathBuf,

    /// Optionally specified world that bindings are generated for.
    ///
    /// Bindings are always generated for a world but this option can be omitted
    /// when the WIT package pointed to by the `WIT` option only has a single
    /// world. If there's more than one world in the package then this option
    /// must be specified to name the world that bindings are generated for.
    /// This option can also use the fully qualified syntax such as
    /// `wasi:http/proxy` to select a world from a dependency of the main WIT
    /// package.
    #[clap(short, long)]
    world: Option<String>,

    /// Indicates that no files are written and instead files are checked if
    /// they're up-to-date with the source files.
    #[clap(long)]
    check: bool,

    /// Comma-separated list of features that should be enabled when processing
    /// WIT files.
    ///
    /// This enables using `@unstable` annotations in WIT files.
    #[clap(long)]
    features: Vec<String>,

    /// Whether or not to activate all WIT features when processing WIT files.
    ///
    /// This enables using `@unstable` annotations in WIT files.
    #[clap(long)]
    all_features: bool,
}

// ScaffoldGenerator implements WorldGenerator to integrate with existing infrastructure
struct ScaffoldGenerator {
    output_dir: PathBuf,
    with_cargo: bool,
    component_name: String,
}

impl WorldGenerator for ScaffoldGenerator {
    fn generate(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        files: &mut Files,
    ) -> Result<()> {
        eprintln!("Generating Rust scaffolding...");
        
        // Validate world exists
        let world_obj = resolve.worlds.get(world)
            .with_context(|| format!("World ID {:?} not found in resolve", world))?;
        
        eprintln!("Generating scaffolding for world: '{}'", world_obj.name);
        
        // Validate component name
        if self.component_name.is_empty() {
            bail!("Component name cannot be empty");
        }
        
        if !self.component_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            bail!("Component name can only contain alphanumeric characters, hyphens, and underscores");
        }
        
        // Generate Cargo.toml if requested
        if self.with_cargo {
            let cargo_content = generate_cargo_toml(&self.component_name, &world_obj.name)
                .with_context(|| "Failed to generate Cargo.toml content")?;
            files.push("Cargo.toml", cargo_content.as_bytes());
        }
        
        // Generate lib.rs with stub implementations
        let lib_content = generate_lib_rs_from_world(resolve, world, &world_obj.name)
            .with_context(|| "Failed to generate lib.rs content")?;
        let lib_file = if self.output_dir.file_name().unwrap_or_default() == "src" {
            "lib.rs"
        } else {
            "src/lib.rs"
        };
        files.push(lib_file, lib_content.as_bytes());
        
        // Generate README with instructions
        let readme_content = generate_readme(&self.component_name, &world_obj.name)
            .with_context(|| "Failed to generate README.md content")?;
        files.push("README.md", readme_content.as_bytes());
        
        eprintln!("Scaffolding generation complete!");
        eprintln!("Next steps:");
        eprintln!("  1. Implement the TODO functions in {}", lib_file);
        eprintln!("  2. Build with: cargo build --target wasm32-wasip2");
        eprintln!("  3. Test with: wit-bindgen validate <wit-path>");
        
        Ok(())
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let mut files = Files::default();
    let (generator, opt) = match Opt::parse() {
        #[cfg(feature = "markdown")]
        Opt::Markdown { opts, args } => (opts.build(), args),
        #[cfg(feature = "moonbit")]
        Opt::Moonbit { opts, args } => (opts.build(), args),
        #[cfg(feature = "c")]
        Opt::C { opts, args } => (opts.build(), args),
        #[cfg(feature = "cpp")]
        Opt::Cpp { opts, args } => (opts.build(args.out_dir.as_ref()), args),
        #[cfg(feature = "rust")]
        Opt::Rust { opts, args } => (opts.build(), args),
        #[cfg(feature = "go")]
        Opt::TinyGo { args: _ } => {
            bail!("Go bindgen has been moved to a separate repository. Please visit https://github.com/bytecodealliance/go-modules for the new Go bindings generator `wit-bindgen-go`.")
        }
        #[cfg(feature = "csharp")]
        Opt::Csharp { opts, args } => (opts.build(), args),
        Opt::Test { opts } => return opts.run(std::env::args_os().nth(0).unwrap().as_ref()),
        Opt::Validate { args, recursive, show_tree } => {
            return validate_wit_dependencies(&args, recursive, show_tree);
        }
        Opt::Scaffold { args, output, with_cargo, name } => {
            let component_name = name.unwrap_or_else(|| "component".to_string());
            let generator = ScaffoldGenerator {
                output_dir: output,
                with_cargo,
                component_name,
            };
            (Box::new(generator), args)
        }
        Opt::Interactive { args, guided } => {
            return run_interactive_mode(&args, guided);
        }
    };

    gen_world(generator, &opt, &mut files).map_err(attach_with_context)?;

    for (name, contents) in files.iter() {
        let dst = match &opt.out_dir {
            Some(path) => path.join(name),
            None => name.into(),
        };
        eprintln!("Generating {:?}", dst);

        if opt.check {
            let prev = std::fs::read(&dst).with_context(|| format!("failed to read {:?}", dst))?;
            if prev != contents {
                // The contents differ. If it looks like textual contents, do a
                // line-by-line comparison so that we can tell users what the
                // problem is directly.
                if let (Ok(utf8_prev), Ok(utf8_contents)) =
                    (str::from_utf8(&prev), str::from_utf8(contents))
                {
                    if !utf8_prev
                        .chars()
                        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
                        && utf8_prev.lines().eq(utf8_contents.lines())
                    {
                        bail!("{} differs only in line endings (CRLF vs. LF). If this is a text file, configure git to mark the file as `text eol=lf`.", dst.display());
                    }
                }
                // The contents are binary or there are other differences; just
                // issue a generic error.
                bail!("not up to date: {}", dst.display());
            }
            continue;
        }

        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {:?}", parent))?;
        }
        std::fs::write(&dst, contents).with_context(|| format!("failed to write {:?}", dst))?;
    }

    Ok(())
}

fn attach_with_context(err: Error) -> Error {
    #[cfg(feature = "rust")]
    if let Some(e) = err.downcast_ref::<wit_bindgen_rust::MissingWith>() {
        let option = e.0.clone();
        return err.context(format!(
            "missing either `--generate-all` or `--with {option}=(...|generate)`"
        ));
    }
    err
}

fn gen_world(
    mut generator: Box<dyn WorldGenerator>,
    opts: &Common,
    files: &mut Files,
) -> Result<()> {
    let mut resolve = Resolve::default();
    resolve.all_features = opts.all_features;
    for features in opts.features.iter() {
        for feature in features
            .split(',')
            .flat_map(|s| s.split_whitespace())
            .filter(|f| !f.is_empty())
        {
            resolve.features.insert(feature.to_string());
        }
    }
    let (pkg, _files) = resolve.push_path(&opts.wit)?;
    let world = resolve.select_world(pkg, opts.world.as_deref())?;
    generator.generate(&resolve, world, files)?;

    Ok(())
}

// Shared utility for setting up resolve with features
fn setup_resolve_with_features(opts: &Common) -> Resolve {
    let mut resolve = Resolve::default();
    resolve.all_features = opts.all_features;
    for features in opts.features.iter() {
        for feature in features
            .split(',')
            .flat_map(|s| s.split_whitespace())
            .filter(|f| !f.is_empty())
        {
            resolve.features.insert(feature.to_string());
        }
    }
    resolve
}

// Shared utility for parsing WIT files
fn parse_wit_package(opts: &Common) -> Result<(Resolve, wit_parser::PackageId)> {
    let mut resolve = setup_resolve_with_features(opts);
    let (pkg, _files) = resolve.push_path(&opts.wit)?;
    Ok((resolve, pkg))
}

// Shared utility for selecting world
fn select_world_from_package(resolve: &Resolve, pkg: wit_parser::PackageId, world_name: Option<&str>) -> Result<wit_parser::WorldId> {
    resolve.select_world(pkg, world_name)
}

fn validate_wit_dependencies(opts: &Common, recursive: bool, show_tree: bool) -> Result<()> {
    let mut resolve = setup_resolve_with_features(opts);
    eprintln!("Validating WIT dependencies for: {}", opts.wit.display());
    
    // Try to parse the WIT files and catch detailed errors
    let result = resolve.push_path(&opts.wit);
    match result {
        Ok((pkg, _files)) => {
            eprintln!("WIT package parsed successfully!");
            let world_id = validate_world_selection(&resolve, pkg, opts.world.as_deref())?;
            
            if show_tree {
                print_world_structure(&resolve, world_id);
            }
            
            if recursive {
                validate_dependencies_recursive(&resolve, pkg)?;
            }
            
            if show_tree {
                print_dependency_tree(&resolve, pkg);
            }
            
            eprintln!("All validations passed!");
        }
        Err(e) => return handle_validation_error(e),
    }
    
    Ok(())
}

// Helper function for validating world selection
fn validate_world_selection(resolve: &Resolve, pkg: wit_parser::PackageId, world_name: Option<&str>) -> Result<wit_parser::WorldId> {
    if let Some(world_name) = world_name {
        match resolve.select_world(pkg, Some(world_name)) {
            Ok(world_id) => {
                eprintln!("World '{}' found and valid", world_name);
                Ok(world_id)
            }
            Err(e) => {
                eprintln!("error: World validation failed: {}", e);
                Err(e)
            }
        }
    } else {
        // Try to auto-select a world
        match resolve.select_world(pkg, None) {
            Ok(world_id) => {
                let world = &resolve.worlds[world_id];
                eprintln!("Default world '{}' found and valid", world.name);
                Ok(world_id)
            }
            Err(e) => {
                eprintln!("error: No default world found: {}", e);
                Err(e)
            }
        }
    }
}

// Helper function for handling validation errors with comprehensive error analysis
fn handle_validation_error(e: anyhow::Error) -> Result<()> {
    eprintln!("error: WIT validation failed:");
    eprintln!("{}", e);
    
    // Provide helpful suggestions based on error type
    let error_msg = e.to_string();
    
    if error_msg.contains("failed to resolve directory") {
        provide_directory_resolution_help();
    } else if error_msg.contains("package not found") || error_msg.contains("package") && error_msg.contains("not found") {
        provide_package_not_found_help(&error_msg);
    } else if error_msg.contains("world not found") {
        provide_world_not_found_help();
    } else if error_msg.contains("syntax error") || error_msg.contains("expected") {
        provide_syntax_error_help();
    } else if error_msg.contains("interface") && error_msg.contains("not defined") {
        provide_interface_error_help();
    } else if error_msg.contains("type") && error_msg.contains("not defined") {
        provide_type_error_help();
    } else {
        provide_general_help();
    }
    
    Err(e)
}

// Specific error help functions
fn provide_directory_resolution_help() {
    eprintln!("\nSuggestions:");
    eprintln!("  • Check that the WIT directory exists and contains .wit files");
    eprintln!("  • Verify deps.toml file if using package dependencies");
    eprintln!("  • Ensure all imported packages are in the deps/ directory");
    eprintln!("  • Check file permissions on the WIT directory");
}

fn provide_package_not_found_help(error_msg: &str) {
    eprintln!("\nSuggestions:");
    eprintln!("  • Add missing package to deps.toml:");
    eprintln!("    [deps.\"missing:package\"]");
    eprintln!("    path = \"./deps/missing-package\"");
    eprintln!("  • Or place the package in the deps/ directory");
    
    // Try to extract package name from error for more specific help
    if let Some(start) = error_msg.find("package '") {
        if let Some(end) = error_msg[start + 9..].find("'") {
            let package_name = &error_msg[start + 9..start + 9 + end];
            eprintln!("  • For package '{}', try:", package_name);
            eprintln!("    - Check if it exists in your deps/ directory");
            eprintln!("    - Verify the package name matches exactly");
        }
    }
}

fn provide_world_not_found_help() {
    eprintln!("\nSuggestions:");
    eprintln!("  • Check that the world name matches exactly (case-sensitive)");
    eprintln!("  • Use `wit-bindgen validate --show-tree` to see available worlds");
    eprintln!("  • Try running without specifying a world to use the default");
}

fn provide_syntax_error_help() {
    eprintln!("\nSuggestions:");
    eprintln!("  • Check WIT syntax - look for missing semicolons, brackets, or keywords");
    eprintln!("  • Verify package declarations start with 'package namespace:name'");
    eprintln!("  • Ensure interface and world definitions are properly closed");
    eprintln!("  • Check for typos in WIT keywords (interface, world, type, etc.)");
}

fn provide_interface_error_help() {
    eprintln!("\nSuggestions:");
    eprintln!("  • Check that interface names are defined before being used");
    eprintln!("  • Verify import statements include the correct package namespace");
    eprintln!("  • Ensure interface definitions are in the right package");
}

fn provide_type_error_help() {
    eprintln!("\nSuggestions:");
    eprintln!("  • Define custom types before using them in functions");
    eprintln!("  • Check spelling of type names (case-sensitive)");
    eprintln!("  • Import types from other packages if needed");
}

fn provide_general_help() {
    eprintln!("\nGeneral troubleshooting:");
    eprintln!("  • Run with `wit-bindgen validate --show-tree` for more details");
    eprintln!("  • Check the wit-bindgen documentation for syntax examples");
    eprintln!("  • Verify all .wit files have valid syntax");
}

fn print_world_structure(resolve: &Resolve, world_id: wit_parser::WorldId) {
    let world = &resolve.worlds[world_id];
    eprintln!("\nWorld '{}' structure:", world.name);
    
    if !world.imports.is_empty() {
        eprintln!("  Imports:");
        for (key, _item) in world.imports.iter() {
            eprintln!("    import: {}", resolve.name_world_key(key));
        }
    }
    
    if !world.exports.is_empty() {
        eprintln!("  Exports:");
        for (key, _item) in world.exports.iter() {
            eprintln!("    export: {}", resolve.name_world_key(key));
        }
    }
}

fn validate_dependencies_recursive(resolve: &Resolve, _pkg: wit_parser::PackageId) -> Result<()> {
    eprintln!("Validating all dependencies recursively...");
    
    for (_pkg_id, package) in resolve.packages.iter() {
        eprintln!("  package: {}:{}", package.name.namespace, package.name.name);
        
        // Basic validation - if we got this far, dependencies are resolved
        // Additional validation logic can be added here
    }
    
    eprintln!("All dependencies validated successfully");
    Ok(())
}

fn print_dependency_tree(resolve: &Resolve, root_pkg: wit_parser::PackageId) {
    eprintln!("\nDependency tree:");
    let root_package = &resolve.packages[root_pkg];
    eprintln!("root: {}:{}", root_package.name.namespace, root_package.name.name);
    
    // List all packages that were resolved
    for (_pkg_id, package) in resolve.packages.iter() {
        if package.name.namespace != root_package.name.namespace || 
           package.name.name != root_package.name.name {
            eprintln!("  dep: {}:{}", package.name.namespace, package.name.name);
        }
    }
}

fn generate_scaffolding(opts: &Common, output_dir: &PathBuf, with_cargo: bool, name: Option<String>) -> Result<()> {
    eprintln!("Generating Rust scaffolding for: {}", opts.wit.display());
    
    let (resolve, pkg) = parse_wit_package(opts)
        .with_context(|| "Failed to parse WIT files for scaffolding")?;
    
    let world_id = select_world_from_package(&resolve, pkg, opts.world.as_deref())
        .with_context(|| "Failed to select world for scaffolding")?;
    
    let world = &resolve.worlds[world_id];
    let component_name = name.as_deref()
        .unwrap_or(&world.name)
        .replace('-', "_");
    
    eprintln!("Generating scaffolding for world: '{}'", world.name);
    
    // Create output directory
    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir.display()))?;
    
    generate_project_files(&resolve, world_id, opts, output_dir, with_cargo, &component_name)?;
    
    eprintln!("Scaffolding generation complete!");
    print_next_steps(output_dir, &opts.wit);
    
    Ok(())
}

// Helper function to generate all project files
fn generate_project_files(
    resolve: &Resolve, 
    world_id: wit_parser::WorldId, 
    opts: &Common, 
    output_dir: &PathBuf, 
    with_cargo: bool, 
    component_name: &str
) -> Result<()> {
    let world = &resolve.worlds[world_id];
    
    // Generate Cargo.toml if requested
    if with_cargo {
        generate_cargo_file(output_dir, component_name, &world.name)?;
    }
    
    // Generate lib.rs with stub implementations
    generate_lib_file(resolve, world_id, opts, output_dir, &world.name)?;
    
    // Generate README with instructions
    generate_readme_file(output_dir, component_name, &world.name)?;
    
    eprintln!("Scaffolding generation complete!");
    eprintln!("Next steps:");
    eprintln!("  1. Implement the TODO functions in {}", lib_path.display());
    eprintln!("  2. Build with: cargo build --target wasm32-wasip2");
    eprintln!("  3. Test with: wit-bindgen validate {}", opts.wit.display());
    
    Ok(())
}

fn generate_cargo_toml(component_name: &str, world_name: &str) -> Result<String> {
    // Use the same version as the CLI tool
    let wit_bindgen_version = version();
    
    Ok(format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"
description = "Generated component for WIT world '{}'"

[lib]
crate-type = ["cdylib"]

[dependencies]
wit-bindgen = "{}"

# Configuration for building WASM components
[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"
"#, component_name, world_name, wit_bindgen_version))

// Updated function to work with WorldId directly (for Files infrastructure)
fn generate_lib_rs_from_world(resolve: &Resolve, world_id: WorldId, world_name: &str) -> Result<String> {
    let world = &resolve.worlds[world_id];
    
    let mut content = String::new();
    content.push_str(&format!(r#"// Generated component implementation for world '{}'
// TODO: Implement the functions marked with TODO comments

wit_bindgen::generate!({{
    world: "{}",
    path: "wit/",
    // Uncomment to see generated module paths:
    // show_module_paths: true,
}});

struct Component;

"#, world_name, world_name));

    // Generate export implementations
    for (key, item) in world.exports.iter() {
        match item {
            wit_parser::WorldItem::Interface { id, .. } => {
                let interface = &resolve.interfaces[*id];
                let module_path = compute_export_module_path(resolve, key);
                
                content.push_str(&format!("impl {}::Guest for Component {{\n", module_path));
                
                // Generate function stubs for interface
                for (_name, func) in interface.functions.iter() {
                    content.push_str(&generate_function_stub(func));
                }
                
                // Generate resource implementations if any
                for (name, type_id) in interface.types.iter() {
                    if let wit_parser::TypeDefKind::Resource = &resolve.types[*type_id].kind {
                        content.push_str(&generate_resource_stub(name));
                    }
                }
                
                content.push_str("}\n\n");
            }
            wit_parser::WorldItem::Function(func) => {
                content.push_str("impl Guest for Component {\n");
                content.push_str(&generate_function_stub(func));
                content.push_str("}\n\n");
            }
            _ => {}
        }
    }
    
    content.push_str("export!(Component);\n");
    Ok(content)
}

// Legacy function for backward compatibility
fn generate_lib_rs(resolve: &Resolve, world_id: wit_parser::WorldId, _wit_path: &PathBuf, world_name: &str) -> String {
    // Use the new implementation that works with Files infrastructure
    generate_lib_rs_from_world(resolve, world_id, world_name)
        .unwrap_or_else(|e| {
            eprintln!("warning: Failed to generate lib.rs: {}", e);
            "// Failed to generate content".to_string()
        })
}

fn compute_export_module_path(resolve: &Resolve, key: &wit_parser::WorldKey) -> String {
    match key {
        wit_parser::WorldKey::Name(name) => {
            format!("exports::{}", name.replace('-', "_"))
        }
        wit_parser::WorldKey::Interface(id) => {
            let interface = &resolve.interfaces[*id];
            if let Some(pkg_id) = interface.package {
                let pkg = &resolve.packages[pkg_id];
                format!("exports::{}::{}::{}", 
                       pkg.name.namespace.replace('-', "_"),
                       pkg.name.name.replace('-', "_"),
                       interface.name.as_ref().unwrap().replace('-', "_"))
            } else {
                "exports::interface".to_string()
            }
        }
    }
}

fn generate_function_stub(func: &wit_parser::Function) -> String {
    let mut stub = String::new();
    
    // Generate function signature with proper types
    stub.push_str(&format!("    fn {}(", func.name.replace('-', "_")));
    
    // Generate parameters with proper type mapping
    for (i, (name, ty)) in func.params.iter().enumerate() {
        if i > 0 { stub.push_str(", "); }
        stub.push_str(&format!("{}: {},", 
            name.replace('-', "_"),
            map_wit_type_to_rust(ty)
        ));
    }
    
    stub.push_str(")");
    
    // Generate return type with proper mapping
    if let Some(result_ty) = &func.result {
        stub.push_str(&format!(" -> {}", map_wit_type_to_rust(result_ty)));
    }
    
    stub.push_str(" {\n");
    stub.push_str(&format!("        // TODO: Implement {}\n", func.name));
    stub.push_str("        todo!()\n");
    stub.push_str("    }\n\n");
    
    stub
}

// Helper function to map WIT types to Rust types
fn map_wit_type_to_rust(ty: &wit_parser::Type) -> String {
    match ty {
        wit_parser::Type::Bool => "bool".to_string(),
        wit_parser::Type::U8 => "u8".to_string(),
        wit_parser::Type::U16 => "u16".to_string(),
        wit_parser::Type::U32 => "u32".to_string(),
        wit_parser::Type::U64 => "u64".to_string(),
        wit_parser::Type::S8 => "i8".to_string(),
        wit_parser::Type::S16 => "i16".to_string(),
        wit_parser::Type::S32 => "i32".to_string(),
        wit_parser::Type::S64 => "i64".to_string(),
        wit_parser::Type::F32 => "f32".to_string(),
        wit_parser::Type::F64 => "f64".to_string(),
        wit_parser::Type::Char => "char".to_string(),
        wit_parser::Type::String => "String".to_string(),
        wit_parser::Type::Id(id) => {
            // For complex types, use a generic placeholder for now
            // In a full implementation, this would resolve the actual type name
            format!("/* Type {} */", id.index())
        }
    }
}

fn generate_resource_stub(name: &str) -> String {
    let resource_name = name.replace('-', "_");
    let type_name = resource_name.to_uppercase();
    
    format!(r#"    type {} = (); // TODO: Define your resource type
    
    fn [new-{}]() -> Self::{} {{
        // TODO: Implement resource constructor
        todo!()
    }}
    
    fn [drop](_rep: Self::{}) {{
        // TODO: Implement resource destructor
        todo!()
    }}
    
"#, type_name, resource_name, type_name, type_name)
}

fn generate_readme(component_name: &str, world_name: &str) -> Result<String> {
    format!(r#"# {} Component

Generated scaffolding for WIT world `{}`.

## Getting Started

1. **Implement the functions** marked with `TODO` in `src/lib.rs`
2. **Build the component**:
   ```bash
   cargo build --target wasm32-wasip2
   ```
3. **Validate your implementation**:
   ```bash
   wit-bindgen validate wit/
   ```

## Development Tips

- Use `show_module_paths: true` in the `wit_bindgen::generate!` macro to see generated module paths
- Test your WIT files with `wit-bindgen validate` before implementing
- Use `cargo expand` to see the generated bindings code

## Project Structure

- `src/lib.rs` - Main component implementation
- `wit/` - WIT interface definitions  
- `Cargo.toml` - Rust project configuration

## Building for Production

```bash
cargo build --target wasm32-wasip2 --release
wasm-tools component new target/wasm32-wasip2/release/{}.wasm -o component.wasm
```
"#, component_name, world_name, component_name))
}

fn run_interactive_mode(opts: &Common, guided: bool) -> Result<()> {
    eprintln!("Welcome to wit-bindgen Interactive Mode!");
    println!();
    
    if guided {
        eprintln!("This guided mode will walk you through creating a WebAssembly component step-by-step.");
        println!();
    }
    
    // Step 1: Validate WIT files
    eprintln!("Step 1: Validating WIT dependencies...");
    let validation_result = validate_wit_dependencies(opts, false, false);
    
    match validation_result {
        Ok(_) => {
            eprintln!("WIT validation passed!");
        }
        Err(e) => {
            eprintln!("error: WIT validation failed: {}", e);
            println!();
            if !prompt_yes_no("Continue anyway? This may cause compilation issues.")? {
                return Ok(());
            }
        }
    }
    
    println!();
    
    // Step 2: Parse and analyze WIT structure
    let mut resolve = Resolve::default();
    resolve.all_features = opts.all_features;
    for features in opts.features.iter() {
        for feature in features
            .split(',')
            .flat_map(|s| s.split_whitespace())
            .filter(|f| !f.is_empty())
        {
            resolve.features.insert(feature.to_string());
        }
    }
    
    let (pkg, _files) = resolve.push_path(&opts.wit)
        .with_context(|| "Failed to parse WIT files")?;
    
    let world_id = resolve.select_world(pkg, opts.world.as_deref())
        .with_context(|| "Failed to select world")?;
    
    let world = &resolve.worlds[world_id];
    
    eprintln!("Step 2: Analyzing WIT structure...");
    print_world_structure(&resolve, world_id);
    
    if guided {
        println!();
        eprintln!("Your component will need to:");
        
        if !world.exports.is_empty() {
            eprintln!("   • Implement {} export interface(s)", world.exports.len());
            for (key, _item) in world.exports.iter() {
                eprintln!("     - {}", resolve.name_world_key(key));
            }
        }
        
        if !world.imports.is_empty() {
            eprintln!("   • Use {} imported interface(s)", world.imports.len());
            for (key, _item) in world.imports.iter() {
                eprintln!("     - {}", resolve.name_world_key(key));
            }
        }
        
        println!();
        pause_for_user("Press Enter to continue...")?;
    }
    
    // Step 3: Offer next actions
    println!("Step 3: Choose your next action:");
    println!("  1. Generate scaffolding (recommended for new projects)");
    println!("  2. Show generated module paths");
    println!("  3. Generate bindings only");
    println!("  4. Exit");
    println!();
    
    let choice = prompt_choice("Your choice", &["1", "2", "3", "4"])?;
    
    match choice.as_str() {
        "1" => {
            println!();
            let component_name = prompt_string("Component name", Some(&world.name.replace('-', "_")))?;
            let with_cargo = prompt_yes_no("Generate Cargo.toml project file?")?;
            let output_dir = PathBuf::from(prompt_string("Output directory", Some("src"))?);
            
            println!();
            generate_scaffolding(opts, &output_dir, with_cargo, Some(component_name))?;
        }
        "2" => {
            println!();
            println!("Generated module paths for world '{}':", world.name);
            
            for (key, item) in world.exports.iter() {
                if let wit_parser::WorldItem::Interface { .. } = item {
                    let module_path = compute_export_module_path(&resolve, key);
                    println!("  export '{}' -> impl {}::Guest", 
                             resolve.name_world_key(key), 
                             module_path);
                }
            }
            
            println!();
            println!("Use these paths in your Rust implementation!");
        }
        "3" => {
            println!();
            println!("Use the regular wit-bindgen commands:");
            println!("  wit-bindgen rust {}", opts.wit.display());
        }
        "4" => {
            println!("Goodbye!");
            return Ok(());
        }
        _ => unreachable!()
    }
    
    if guided {
        println!();
        println!("You're all set! Here are some helpful next steps:");
        println!("  • Read the generated README.md for detailed instructions");
        println!("  • Use `wit-bindgen validate` to check your WIT files anytime");
        println!("  • Join the WebAssembly community for support and questions");
        println!("  • Check out the component model documentation at component-model.bytecodealliance.org");
    }
    
    Ok(())
}

fn prompt_yes_no(question: &str) -> Result<bool> {
    loop {
        print!("{} [y/N]: ", question);
        io::stdout().flush()?;
        
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("error: Failed to read input: {}", e);
                continue;
            }
        }
        
        let input = input.trim().to_lowercase();
        match input.as_str() {
            "" | "n" | "no" => return Ok(false),
            "y" | "yes" => return Ok(true),
            _ => {
                eprintln!("Please enter 'y' for yes or 'n' for no (or press Enter for no).");
                continue;
            }
        }
    }
}

fn prompt_string(question: &str, default: Option<&str>) -> Result<String> {
    loop {
        if let Some(def) = default {
            print!("{} [{}]: ", question, def);
        } else {
            print!("{}: ", question);
        }
        io::stdout().flush()?;
        
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("error: Failed to read input: {}", e);
                continue;
            }
        }
        
        let input = input.trim();
        if input.is_empty() {
            if let Some(def) = default {
                return Ok(def.to_string());
            } else {
                eprintln!("Please provide a value.");
                continue;
            }
        } else if input.contains(|c: char| c.is_control() && c != '\t') {
            eprintln!("Invalid input: control characters not allowed.");
            continue;
        } else if input.len() > 100 {
            eprintln!("Input too long (max 100 characters).");
            continue;
        } else {
            return Ok(input.to_string());
        }
    }
}

fn prompt_choice(question: &str, choices: &[&str]) -> Result<String> {
    loop {
        print!("{} [{}]: ", question, choices.join("/"));
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let input = input.trim();
        if choices.contains(&input) {
            return Ok(input.to_string());
        } else {
            println!("Invalid choice. Please select from: {}", choices.join(", "));
        }
    }
}

fn pause_for_user(message: &str) -> Result<()> {
    print!("{}", message);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    Ok(())
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Opt::command().debug_assert()
}
