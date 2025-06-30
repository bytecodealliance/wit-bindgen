use anyhow::{bail, Context, Error, Result};
use clap::Parser;
use heck::ToUpperCamelCase;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::PathBuf;
use std::str;
use wit_bindgen_core::{wit_parser, Files, WorldGenerator};
use wit_parser::{Resolve, WorldId};

mod modules {
    pub mod package_registry;
    pub mod usage_tracking;
}

use modules::package_registry::with_package_registry;
use modules::usage_tracking::{with_usage_tracker, SkillLevel};

/// Helper for passing VERSION to opt.
/// If CARGO_VERSION_INFO is set, use it, otherwise use CARGO_PKG_VERSION.
fn version() -> &'static str {
    option_env!("CARGO_VERSION_INFO").unwrap_or(env!("CARGO_PKG_VERSION"))
}

fn run_stats_command(show_stats: bool, reset: bool) -> Result<()> {
    if reset {
        // Reset the profile by creating a new default one
        with_usage_tracker(|tracker| {
            *tracker = modules::usage_tracking::UsageTracker::new();
        });
        println!("User profile has been reset to default settings.");
        return Ok(());
    }

    if show_stats {
        with_usage_tracker(|tracker| {
            println!("wit-bindgen Usage Statistics");
            println!("========================");
            println!();

            let skill_level = tracker.get_skill_level();
            println!("Current Skill Level: {:?}", skill_level);
            println!("Preferred Format: {}", tracker.get_preferred_format());
            println!();

            println!("Command Usage Statistics:");
            let stats = tracker.get_usage_stats();
            if stats.is_empty() {
                println!("  No usage data available yet.");
            } else {
                for (command, usage) in stats.iter() {
                    let success_rate = if usage.count > 0 {
                        (usage.successes as f64 / usage.count as f64) * 100.0
                    } else {
                        0.0
                    };
                    println!(
                        "  {}: {} uses, {:.1}% success rate",
                        command, usage.count, success_rate
                    );
                }
            }
            println!();

            // Provide skill-appropriate tips
            match skill_level {
                SkillLevel::Beginner => {
                    println!("Getting started:");
                    println!("  - Try: wit-bindgen docs for comprehensive documentation");
                    println!("  - Use: wit-bindgen validate --auto-deps to fix dependency issues");
                    println!("  - Enable: --enhanced-codegen for detailed stub implementations");
                }
                SkillLevel::Intermediate => {
                    println!("Advanced features:");
                    println!("  - Use deps --sync-check to verify dependency structure");
                    println!("  - Try analyze --format json for automation");
                    println!("  - Consider using --from to add local dependencies");
                }
                SkillLevel::Advanced | SkillLevel::Expert => {
                    println!("Expert features:");
                    println!("  - Integrate JSON output into CI/CD pipelines");
                    println!("  - Use docs command for API schemas");
                    println!("  - Consider contributing to wit-bindgen registry");
                }
            }
        });
    } else {
        println!("wit-bindgen usage statistics");
        println!("Usage:");
        println!("  --show-stats    Show usage statistics and skill assessment");
        println!("  --reset         Reset user profile to defaults");
    }

    Ok(())
}

fn run_registry_command(
    search: Option<String>,
    analyze_deps: bool,
    recommend: Option<String>,
    health: bool,
    update: bool,
    format: OutputFormat,
) -> Result<()> {
    if update {
        println!("Updating package registry index...");
        with_package_registry(|registry| registry.update_package_index())?;
        println!("Package index updated successfully.");
        return Ok(());
    }

    if health {
        let report = with_package_registry(|registry| registry.generate_registry_report());
        match format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            OutputFormat::Human => {
                println!("WebAssembly Package Registry Report");
                println!("======================================");
                println!();
                println!("Total Packages: {}", report.total_packages);
                println!("Average Health Score: {}/100", report.average_health_score);
                println!();
                println!("Package Categories:");
                for (category, count) in &report.category_distribution {
                    println!("  - {}: {} packages", category, count);
                }
                println!();
                println!("Top Packages by Health:");
                for (i, package) in report.top_packages.iter().enumerate() {
                    println!(
                        "  {}. {} (Score: {}/100)",
                        i + 1,
                        package.name,
                        package.metrics.health_score
                    );
                }
            }
        }
        return Ok(());
    }

    if let Some(query) = search {
        let results = with_package_registry(|registry| registry.search_packages(&query));
        match format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
            OutputFormat::Human => {
                println!("Search Results for '{}'", query);
                println!("================================");
                println!();
                if results.is_empty() {
                    println!("No packages found matching your search.");
                } else {
                    for package in &results {
                        println!("Package: {}", package.name);
                        println!("   ID: {}", package.id);
                        println!("   Description: {}", package.description);
                        println!("   Health Score: {}/100", package.metrics.health_score);
                        println!("   Categories: {}", package.categories.join(", "));
                        println!();
                    }
                }
            }
        }
        return Ok(());
    }

    if let Some(categories) = recommend {
        let category_list: Vec<String> = categories
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        let recommendations =
            with_package_registry(|registry| registry.get_package_recommendations(&category_list));
        match format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&recommendations)?);
            }
            OutputFormat::Human => {
                println!("Package Recommendations for: {}", categories);
                println!("===============================================");
                println!();
                if recommendations.is_empty() {
                    println!("No recommendations found for the specified categories.");
                } else {
                    for (i, package) in recommendations.iter().enumerate() {
                        println!("{}. {}", i + 1, package.name);
                        println!("   ID: {}", package.id);
                        println!("   Description: {}", package.description);
                        println!("   Health Score: {}/100", package.metrics.health_score);
                        println!("   Popularity: {}/100", package.metrics.popularity_score);
                        println!();
                    }
                }
            }
        }
        return Ok(());
    }

    if analyze_deps {
        // This would analyze dependencies from a WIT file, but for now show general help
        println!("Dependency Analysis");
        println!("=====================");
        println!();
        println!("To analyze dependencies for a specific WIT file, use:");
        println!("  wit-bindgen registry --analyze-deps --wit-file <path>");
        println!();
        println!("For now, showing ecosystem overview:");
        let report = with_package_registry(|registry| registry.generate_registry_report());
        println!("Total packages available: {}", report.total_packages);
        println!(
            "Average ecosystem health: {}/100",
            report.average_health_score
        );
        return Ok(());
    }

    // Default: show ecosystem command help
    println!("wit-bindgen Package Registry");
    println!("=====================================");
    println!();
    println!("Commands:");
    println!("  --search <query>          Search packages by keyword");
    println!("  --recommend <categories>  Get package recommendations (comma-separated)");
    println!("  --health                  Show ecosystem health report");
    println!("  --update                  Update package index");
    println!("  --analyze-deps            Analyze dependency compatibility");
    println!("  --format <format>         Output format (human, json)");
    println!();
    println!("Examples:");
    println!("  wit-bindgen registry --search http");
    println!("  wit-bindgen registry --recommend wasi,io,networking");
    println!("  wit-bindgen registry --health --format json");

    Ok(())
}

fn show_api_docs() -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "wit_bindgen_api_documentation": {
                "version": version(),
                "overview": {
                    "purpose": "WebAssembly Interface Types (WIT) binding generator with enhanced dependency management",
                    "key_concepts": {
                        "wit_files": "Define component interfaces using WebAssembly Interface Types",
                        "dependency_resolution": "Resolved from deps/ directory in ALPHABETICAL ORDER (critical!)",
                        "worlds": "Define component boundaries with imports/exports",
                        "packages": "Group related interfaces together",
                        "components": "WebAssembly modules that implement worlds"
                    }
                },
                "directory_structure": {
                    "pattern": "my-component/",
                    "files": {
                        "component.wit": "Main world definition",
                        "deps/": {
                            "description": "Dependencies directory (ALPHABETICAL ORDER MATTERS!)",
                            "examples": {
                                "single_file": "wasi-cli.wit",
                                "directory_package": "wasi-http/ (contains multiple .wit files)"
                            }
                        }
                    },
                    "critical_note": "wit-parser processes deps/ entries in alphabetical order. This affects resolution!"
                },
                "dependency_resolution": {
                    "mechanism": "Directory-based scanning (NOT deps.toml files)",
                    "process": [
                        "1. Scan deps/ directory",
                        "2. Sort entries alphabetically by filename/dirname",
                        "3. Parse each entry (file or directory)",
                        "4. Build dependency graph"
                    ],
                    "alphabetical_ordering": {
                        "importance": "CRITICAL - affects resolution order",
                        "example": ["a-package.wit", "b-package/", "z-interface.wit"]
                    }
                },
                "commands": {
                    "for_ai_agents": {
                        "analyze": {
                            "purpose": "Deep semantic analysis with JSON output",
                            "usage": "wit-bindgen analyze --format json <wit-file>",
                            "output": "Structured analysis including type mappings, dependencies, implementation guidance"
                        },
                        "validate": {
                            "purpose": "Comprehensive validation with structured output",
                            "usage": "wit-bindgen validate --analyze --format json <wit-file>",
                            "features": ["syntax validation", "dependency checking", "semantic analysis"]
                        },
                        "deps": {
                            "purpose": "Advanced dependency management",
                            "subcommands": {
                                "sync_check": "wit-bindgen deps --sync-check --format json <wit-file>",
                                "order_fix": "wit-bindgen deps --order-fix <wit-file>",
                                "add_local": "wit-bindgen deps --add <package> --from <path> <wit-file>"
                            }
                        }
                    },
                    "for_code_generation": {
                        "rust": "wit-bindgen rust <wit-file>",
                        "c": "wit-bindgen c <wit-file>",
                        "scaffold": "wit-bindgen scaffold --with-cargo --name <component> <wit-file>"
                    }
                },
                "json_output_schemas": {
                    "analyze_command": {
                        "valid": "boolean",
                        "worlds": ["array of world analysis"],
                        "type_mappings": "object mapping WIT types to target language types",
                        "dependencies": ["array of dependency info"],
                        "diagnostics": ["array of diagnostic messages"],
                        "implementation_guide": "object with implementation suggestions",
                        "semantic_analysis": "detailed semantic information"
                    },
                    "validate_command": {
                        "valid": "boolean",
                        "errors": ["array of validation errors"],
                        "warnings": ["array of warnings"],
                        "dependency_tree": "object representing dependency relationships"
                    },
                    "deps_command": {
                        "dependencies_found": ["array of detected dependencies"],
                        "missing_dependencies": ["array of missing deps"],
                        "ordering_issues": ["array of alphabetical ordering problems"],
                        "sync_status": "object describing sync between WIT imports and deps/"
                    }
                },
                "common_workflows": {
                    "new_component_development": [
                        "1. wit-bindgen scaffold --with-cargo --name my-component component.wit",
                        "2. wit-bindgen deps --sync-check component.wit",
                        "3. Implement TODO functions in generated code",
                        "4. wit-bindgen validate --analyze component.wit"
                    ],
                    "dependency_management": [
                        "1. wit-bindgen deps --add wasi:http --from /path/to/wasi-http component.wit",
                        "2. wit-bindgen deps --order-fix component.wit",
                        "3. wit-bindgen deps --sync-check --format json component.wit"
                    ],
                    "ai_analysis": [
                        "1. wit-bindgen analyze --templates --format json component.wit",
                        "2. Parse JSON output for semantic information",
                        "3. Use implementation_guide for code generation hints"
                    ]
                },
                "error_handling": {
                    "dependency_resolution_errors": {
                        "alphabetical_ordering": "Use --order-fix to resolve",
                        "missing_dependencies": "Use --add with --from to copy local deps",
                        "sync_issues": "Use --sync-check to identify mismatches"
                    },
                    "validation_errors": {
                        "syntax_errors": "Check WIT file syntax",
                        "type_errors": "Verify interface definitions",
                        "world_errors": "Check import/export consistency"
                    }
                },
                "ai_agent_best_practices": {
                    "always_use_json_format": "Add --format json to get structured output",
                    "check_alphabetical_order": "Dependencies must be alphabetically ordered",
                    "validate_after_changes": "Always validate after modifying deps/",
                    "use_sync_check": "Verify deps/ matches WIT imports",
                    "leverage_semantic_analysis": "Use analyze command for deep understanding"
                },
                "examples": {
                    "basic_wit_file": {
                        "content": "package my:component;\\n\\nworld my-world {\\n  import wasi:cli/environment@0.2.0;\\n  export run: func();\\n}",
                        "explanation": "Defines a component that imports WASI CLI and exports a run function"
                    },
                    "dependency_structure": {
                        "deps/wasi-cli.wit": "Single file dependency",
                        "deps/wasi-http/types.wit": "Part of directory-based package",
                        "deps/wasi-http/handler.wit": "Another file in same package"
                    }
                }
            }
        }))?
    );
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Human,
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "human" => Ok(OutputFormat::Human),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!(
                "Unknown output format: {}. Valid options: human, json",
                s
            )),
        }
    }
}

// Analysis structures for JSON output
#[derive(Debug, Serialize, Deserialize)]
struct WitAnalysis {
    valid: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    worlds: Vec<WorldAnalysis>,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    type_mappings: std::collections::HashMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dependencies: Vec<DependencyInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    implementation_guide: Option<ImplementationGuide>,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_analysis: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WorldAnalysis {
    name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    exports: Vec<InterfaceInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    imports: Vec<InterfaceInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InterfaceInfo {
    #[serde(rename = "type")]
    interface_type: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_path: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    functions: Vec<FunctionInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FunctionInfo {
    name: String,
    wit_signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    rust_signature: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DependencyInfo {
    package: String,
    namespace: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Diagnostic {
    level: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<String>,
    // Enhanced validation fields
    #[serde(skip_serializing_if = "Option::is_none")]
    error_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    confidence: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    actionable_suggestions: Vec<ActionableSuggestion>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    related_concepts: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ActionableSuggestion {
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    priority: String,
    estimated_success_rate: f32,
    explanation: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Location {
    file: String,
    line: usize,
    column: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct ImplementationGuide {
    required_traits: Vec<String>,
    boilerplate: BoilerplateCode,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    examples: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BoilerplateCode {
    struct_definition: String,
    export_macro: String,
    generate_macro: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ScaffoldReport {
    generated_files: Vec<String>,
    module_structure: Vec<ModuleInfo>,
    required_implementations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModuleInfo {
    path: String,
    #[serde(rename = "type")]
    module_type: String,
}

#[derive(Debug, Parser)]
#[command(
    version = version(),
    about = "WebAssembly Interface Types (WIT) binding generator with enhanced dependency management",
    long_about = "wit-bindgen generates language bindings for WebAssembly components using WIT.\n\nKEY CONCEPTS:\n- WIT files define component interfaces using WebAssembly Interface Types\n- Dependencies are resolved from deps/ directory (alphabetical order matters!)\n- Worlds define component boundaries with imports/exports\n- Packages group related interfaces together\n\nDEPENDENCY STRUCTURE:\n  my-component/\n  ├── component.wit     # Main world definition\n  └── deps/             # Dependencies (alphabetical!)\n      ├── wasi-cli.wit  # Single file dependency\n      └── wasi-http/    # Directory-based package\n          ├── types.wit\n          └── handler.wit\n\nFor automation: Use --format json on most commands for structured output.\nUse 'wit-bindgen docs' for comprehensive API documentation."
)]
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
    ///
    /// Checks WIT syntax, dependency resolution, and component integrity.
    /// Use --analyze for enhanced validation with structured output.
    Validate {
        #[clap(flatten)]
        args: Common,

        /// Check all dependencies recursively
        #[clap(long)]
        recursive: bool,

        /// Show dependency tree structure
        #[clap(long)]
        show_tree: bool,

        /// Output format (human, json)
        #[clap(long, default_value = "human")]
        format: OutputFormat,

        /// Include detailed analysis data (useful for automation)
        #[clap(long)]
        analyze: bool,

        /// Auto-discover dependencies by scanning for missing packages
        #[clap(long)]
        auto_deps: bool,
    },

    /// Generate working stub implementations from WIT definitions
    ///
    /// Creates complete project structure with Cargo.toml, lib.rs, and README.
    /// Generated code includes TODO markers for implementation.
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

        /// Output format for results (human, json)
        #[clap(long, default_value = "human")]
        format: OutputFormat,

        /// Generate detailed analysis report
        #[clap(long)]
        report: bool,
    },

    /// Interactive guided implementation mode
    ///
    /// Step-by-step wizard for WIT implementation.
    /// Use --guided for beginner-friendly explanations.
    Interactive {
        #[clap(flatten)]
        args: Common,

        /// Start in guided mode for beginners
        #[clap(long)]
        guided: bool,
    },

    /// Analyze WIT files with enhanced tooling support
    ///
    /// Provides detailed semantic analysis, type mappings, and implementation guidance.
    /// Default output is JSON format optimized for tooling and automation.
    Analyze {
        #[clap(flatten)]
        args: Common,

        /// Include implementation templates
        #[clap(long)]
        templates: bool,

        /// Output format (defaults to json for automation)
        #[clap(long, default_value = "json")]
        format: OutputFormat,
    },

    /// Manage and analyze WIT dependencies
    ///
    /// Dependencies are resolved from deps/ directory in alphabetical order.
    /// Use --add with --from to copy local files/directories.
    /// Use --sync-check to validate deps/ matches WIT imports.
    Deps {
        #[clap(flatten)]
        args: Common,

        /// Check for missing dependencies
        #[clap(long = "check-deps")]
        check_deps: bool,

        /// Generate deps.toml template
        #[clap(long)]
        generate: bool,

        /// Add a specific dependency
        #[clap(long)]
        add: Option<String>,

        /// Copy dependency from local file or directory (use with --add)
        #[clap(long)]
        from: Option<String>,

        /// Auto-fix broken dependency paths by scanning for missing packages
        #[clap(long)]
        fix: bool,

        /// Check if deps/ directory structure matches WIT imports and is properly synchronized
        #[clap(long = "sync-check")]
        sync_check: bool,

        /// Fix alphabetical ordering issues in deps/ directory
        #[clap(long = "order-fix")]
        order_fix: bool,

        /// Output format
        #[clap(long, default_value = "human")]
        format: OutputFormat,
    },

    /// API documentation in structured format
    ///
    /// Provides comprehensive technical documentation and examples
    /// in machine-readable JSON format for tooling integration.
    #[command(name = "docs")]
    Docs,

    /// Display usage statistics
    #[command(name = "stats")]
    Stats {
        /// Show usage statistics and skill assessment
        #[clap(long)]
        show_stats: bool,

        /// Reset user profile and learning data
        #[clap(long)]
        reset: bool,
    },

    /// Package registry operations
    ///
    /// Search and analyze packages in the WebAssembly package registry,
    /// check compatibility, and get recommendations.
    #[command(name = "registry")]
    Registry {
        /// Search packages by keyword
        #[clap(long)]
        search: Option<String>,

        /// Analyze compatibility of dependencies
        #[clap(long)]
        analyze_deps: bool,

        /// Get package recommendations for categories
        #[clap(long)]
        recommend: Option<String>,

        /// Show ecosystem health report
        #[clap(long)]
        health: bool,

        /// Update package index
        #[clap(long)]
        update: bool,

        /// Output format (human, json)
        #[clap(long, default_value = "human")]
        format: OutputFormat,
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
    fn generate(&mut self, resolve: &Resolve, world: WorldId, files: &mut Files) -> Result<()> {
        eprintln!("Generating Rust scaffolding...");

        // Validate world exists
        let world_obj = resolve
            .worlds
            .get(world)
            .with_context(|| format!("World ID {:?} not found in resolve", world))?;

        eprintln!("Generating scaffolding for world: '{}'", world_obj.name);

        // Validate component name
        if self.component_name.is_empty() {
            bail!("Component name cannot be empty");
        }

        if !self
            .component_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            bail!(
                "Component name can only contain alphanumeric characters, hyphens, and underscores"
            );
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

    fn import_interface(
        &mut self,
        _resolve: &Resolve,
        _name: &wit_parser::WorldKey,
        _iface: wit_parser::InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        // Scaffolding doesn't need to handle imports
        Ok(())
    }

    fn export_interface(
        &mut self,
        _resolve: &Resolve,
        _name: &wit_parser::WorldKey,
        _iface: wit_parser::InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        // Scaffolding doesn't need to handle exports
        Ok(())
    }

    fn import_funcs(
        &mut self,
        _resolve: &Resolve,
        _world: WorldId,
        _funcs: &[(&str, &wit_parser::Function)],
        _files: &mut Files,
    ) {
        // Scaffolding doesn't need to handle import functions
    }

    fn export_funcs(
        &mut self,
        _resolve: &Resolve,
        _world: WorldId,
        _funcs: &[(&str, &wit_parser::Function)],
        _files: &mut Files,
    ) -> Result<()> {
        // Scaffolding doesn't need to handle export functions
        Ok(())
    }

    fn import_types(
        &mut self,
        _resolve: &Resolve,
        _world: WorldId,
        _types: &[(&str, wit_parser::TypeId)],
        _files: &mut Files,
    ) {
        // Scaffolding doesn't need to handle import types
    }

    fn finish(&mut self, _resolve: &Resolve, _world: WorldId, _files: &mut Files) -> Result<()> {
        // All file generation is done in generate()
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
        #[cfg(feature = "csharp")]
        Opt::Csharp { opts, args } => (opts.build(), args),
        Opt::Test { opts } => return opts.run(std::env::args_os().nth(0).unwrap().as_ref()),
        Opt::Validate {
            args,
            recursive,
            show_tree,
            format,
            analyze,
            auto_deps,
        } => {
            return validate_wit_dependencies(
                &args, recursive, show_tree, format, analyze, auto_deps,
            );
        }
        Opt::Scaffold {
            args,
            output,
            with_cargo,
            name,
            format,
            report,
        } => {
            return generate_scaffolding_with_format(
                &args, &output, with_cargo, name, format, report,
            );
        }
        Opt::Interactive { args, guided } => {
            return run_interactive_mode(&args, guided);
        }
        Opt::Analyze {
            args,
            templates,
            format,
        } => {
            let result = run_analyze_command(&args, templates, format);
            with_usage_tracker(|tracker| {
                tracker.record_command_usage("analyze", result.is_ok());
            });
            return result;
        }
        Opt::Deps {
            args,
            check_deps,
            generate,
            add,
            from,
            fix,
            sync_check,
            order_fix,
            format,
        } => {
            let command_name = if sync_check {
                "deps --sync-check"
            } else if generate {
                "deps --generate"
            } else if add.is_some() {
                "deps --add"
            } else {
                "deps"
            };
            let result = run_deps_command(
                &args, check_deps, generate, add, from, fix, sync_check, order_fix, format,
            );
            with_usage_tracker(|tracker| {
                tracker.record_command_usage(command_name, result.is_ok());
            });
            return result;
        }
        Opt::Docs => {
            with_usage_tracker(|tracker| {
                tracker.record_command_usage("docs", true);
            });
            return show_api_docs();
        }
        Opt::Stats { show_stats, reset } => {
            return run_stats_command(show_stats, reset);
        }
        Opt::Registry {
            search,
            analyze_deps,
            recommend,
            health,
            update,
            format,
        } => {
            let result =
                run_registry_command(search, analyze_deps, recommend, health, update, format);
            with_usage_tracker(|tracker| {
                tracker.record_command_usage("registry", result.is_ok());
            });
            return result;
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

fn validate_wit_dependencies(
    opts: &Common,
    recursive: bool,
    show_tree: bool,
    format: OutputFormat,
    analyze: bool,
    auto_deps: bool,
) -> Result<()> {
    // Record command usage
    let command_name = if auto_deps {
        "validate --auto-deps"
    } else {
        "validate"
    };

    let result =
        validate_wit_dependencies_impl(opts, recursive, show_tree, format, analyze, auto_deps);

    // Record success/failure and error patterns for learning
    with_usage_tracker(|tracker| {
        match &result {
            Ok(_) => tracker.record_command_usage(command_name, true),
            Err(e) => {
                tracker.record_command_usage(command_name, false);

                // Extract error type for pattern learning
                let error_str = e.to_string();
                if error_str.contains("package") && error_str.contains("not found") {
                    tracker.record_error_pattern("package_not_found");
                } else if error_str.contains("syntax") || error_str.contains("expected") {
                    tracker.record_error_pattern("parse_error");
                } else if error_str.contains("world") && error_str.contains("not found") {
                    tracker.record_error_pattern("world_not_found");
                }
            }
        }
    });

    result
}

fn validate_wit_dependencies_impl(
    opts: &Common,
    recursive: bool,
    show_tree: bool,
    format: OutputFormat,
    analyze: bool,
    auto_deps: bool,
) -> Result<()> {
    let mut resolve = setup_resolve_with_features(opts);

    match format {
        OutputFormat::Human => {
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
                Err(e) => {
                    if auto_deps {
                        eprintln!("Validation failed, running dependency analysis...");
                        eprintln!("Error: {}", e);

                        // First, run dependency structure validation
                        let base_dir = opts.wit.parent().unwrap_or(&opts.wit);
                        let scanner = DirectoryDependencyScanner::new(base_dir);

                        eprintln!("\nAnalyzing dependency structure...");
                        match scanner.validate_structure() {
                            Ok(issues) => {
                                if !issues.is_empty() {
                                    eprintln!(
                                        "Found {} dependency structure issues:",
                                        issues.len()
                                    );
                                    for issue in &issues {
                                        let prefix = match issue.severity {
                                            IssueSeverity::Error => "Error:",
                                            IssueSeverity::Warning => "Warning:",
                                            IssueSeverity::Info => "ℹ️  Info:",
                                        };
                                        eprintln!("  {} {}", prefix, issue.message);
                                        eprintln!("     Suggestion: {}", issue.suggestion);
                                    }
                                } else {
                                    eprintln!("Dependency structure looks correct");
                                }

                                let packages = scanner.scan_packages()?;
                                eprintln!("Found {} packages in deps/ directory", packages.len());
                                for package in &packages {
                                    eprintln!(
                                        "  - {} ({} WIT files)",
                                        package.name, package.wit_files
                                    );
                                }
                            }
                            Err(scan_err) => {
                                eprintln!(
                                    "Warning: Could not analyze dependency structure: {}",
                                    scan_err
                                );
                            }
                        }

                        // Try to auto-fix dependencies
                        eprintln!("\nAttempting to auto-fix dependencies...");
                        match fix_dependencies(opts, OutputFormat::Human) {
                            Ok(_) => {
                                eprintln!("\nRetrying validation after auto-fixes...");

                                // Retry validation with a fresh resolver
                                let mut retry_resolve = setup_resolve_with_features(opts);
                                match retry_resolve.push_path(&opts.wit) {
                                    Ok((pkg, _)) => {
                                        eprintln!("Validation succeeded after auto-fixes!");
                                        let world_id = validate_world_selection(
                                            &retry_resolve,
                                            pkg,
                                            opts.world.as_deref(),
                                        )?;

                                        if show_tree {
                                            print_world_structure(&retry_resolve, world_id);
                                            print_dependency_tree(&retry_resolve, pkg);
                                        }
                                    }
                                    Err(retry_err) => {
                                        eprintln!(
                                            "Error: Validation still failed after auto-fixes:"
                                        );
                                        eprintln!("Manual steps may be required:");
                                        eprintln!("  1. Check WIT syntax in your files");
                                        eprintln!(
                                            "  2. Verify package names match expected format"
                                        );
                                        eprintln!("  3. Ensure all dependencies are properly placed in deps/");
                                        return handle_validation_error(retry_err);
                                    }
                                }
                            }
                            Err(fix_err) => {
                                eprintln!("Error: Auto-fix failed: {}", fix_err);
                                eprintln!(
                                    "Manual intervention required - check the suggestions above"
                                );
                                return handle_validation_error(e);
                            }
                        }
                    } else {
                        return handle_validation_error(e);
                    }
                }
            }
        }
        OutputFormat::Json => {
            if auto_deps {
                // Try validation first, then auto-fix if needed
                match analyze_wit_to_json(&mut resolve, opts, recursive, show_tree, analyze) {
                    Ok(analysis) => {
                        println!("{}", serde_json::to_string_pretty(&analysis)?);
                    }
                    Err(e) => {
                        // Attempt auto-fix
                        match fix_dependencies(opts, OutputFormat::Json) {
                            Ok(_) => {
                                // Retry with fresh resolver
                                let mut retry_resolve = setup_resolve_with_features(opts);
                                match analyze_wit_to_json(
                                    &mut retry_resolve,
                                    opts,
                                    recursive,
                                    show_tree,
                                    analyze,
                                ) {
                                    Ok(analysis) => {
                                        let result = serde_json::json!({
                                            "validation_result": analysis,
                                            "auto_fixes_applied": true
                                        });
                                        println!("{}", serde_json::to_string_pretty(&result)?);
                                    }
                                    Err(retry_err) => {
                                        let error_analysis =
                                            create_json_error_analysis(&retry_err, opts);
                                        println!(
                                            "{}",
                                            serde_json::to_string_pretty(&error_analysis)?
                                        );
                                        return Err(retry_err);
                                    }
                                }
                            }
                            Err(_) => {
                                let error_analysis = create_json_error_analysis(&e, opts);
                                println!("{}", serde_json::to_string_pretty(&error_analysis)?);
                                return Err(e);
                            }
                        }
                    }
                }
            } else {
                match analyze_wit_to_json(&mut resolve, opts, recursive, show_tree, analyze) {
                    Ok(analysis) => {
                        println!("{}", serde_json::to_string_pretty(&analysis)?);
                    }
                    Err(e) => {
                        let error_analysis = create_json_error_analysis(&e, opts);
                        println!("{}", serde_json::to_string_pretty(&error_analysis)?);
                        return Err(e);
                    }
                }
            }
        }
    }

    Ok(())
}

// Helper function to analyze WIT and produce JSON output
fn analyze_wit_to_json(
    resolve: &mut Resolve,
    opts: &Common,
    recursive: bool,
    show_tree: bool,
    analyze: bool,
) -> Result<WitAnalysis> {
    let mut analysis = WitAnalysis {
        valid: false,
        worlds: Vec::new(),
        type_mappings: std::collections::HashMap::new(),
        dependencies: Vec::new(),
        diagnostics: Vec::new(),
        implementation_guide: None,
        semantic_analysis: None,
    };

    // Try to parse the WIT files
    match resolve.push_path(&opts.wit) {
        Ok((pkg, _files)) => {
            // Don't set valid=true yet - we need to validate dependencies first
            let parsing_successful = true;

            // Get world information
            if let Ok(world_id) = validate_world_selection(resolve, pkg, opts.world.as_deref()) {
                let world = &resolve.worlds[world_id];
                let mut world_analysis = WorldAnalysis {
                    name: world.name.clone(),
                    exports: Vec::new(),
                    imports: Vec::new(),
                };

                // Analyze exports
                for (key, item) in world.exports.iter() {
                    if let wit_parser::WorldItem::Interface { id, .. } = item {
                        let interface = &resolve.interfaces[*id];
                        let module_path = compute_export_module_path(resolve, key);

                        let mut functions = Vec::new();
                        for (name, func) in interface.functions.iter() {
                            functions.push(FunctionInfo {
                                name: name.clone(),
                                wit_signature: format_wit_function(func),
                                rust_signature: if analyze {
                                    Some(format_rust_function(func, resolve))
                                } else {
                                    None
                                },
                            });
                        }

                        world_analysis.exports.push(InterfaceInfo {
                            interface_type: "interface".to_string(),
                            name: resolve.name_world_key(key),
                            module_path: Some(module_path),
                            functions,
                        });
                    }
                }

                // Analyze imports
                for (key, item) in world.imports.iter() {
                    if let wit_parser::WorldItem::Interface { .. } = item {
                        world_analysis.imports.push(InterfaceInfo {
                            interface_type: "interface".to_string(),
                            name: resolve.name_world_key(key),
                            module_path: None,
                            functions: Vec::new(),
                        });
                    }
                }

                analysis.worlds.push(world_analysis);

                // Add implementation guide if requested
                if analyze {
                    analysis.implementation_guide =
                        Some(create_implementation_guide(resolve, world_id));

                    // TODO: Re-enable semantic analysis when module is restored
                    // Perform comprehensive semantic analysis
                    // let semantic_analyzer = SemanticAnalyzer::new(resolve.clone());
                    // let semantic_results = semantic_analyzer.analyze_package(pkg);

                    // Convert to JSON for inclusion in analysis
                    // if let Ok(semantic_json) = serde_json::to_value(&semantic_results) {
                    //     analysis.semantic_analysis = Some(semantic_json);
                    // }
                }
            }

            // Add dependencies
            if recursive || show_tree {
                for (_pkg_id, package) in resolve.packages.iter() {
                    analysis.dependencies.push(DependencyInfo {
                        package: format!("{}:{}", package.name.namespace, package.name.name),
                        namespace: package.name.namespace.clone(),
                        name: package.name.name.clone(),
                    });
                }
            }

            // Add type mappings if analyzing
            if analyze {
                analysis.type_mappings = create_type_mappings();
            }

            // Now perform comprehensive validation
            let mut validation_success = true;

            // Validate world selection
            match validate_world_selection(resolve, pkg, opts.world.as_deref()) {
                Ok(_) => {
                    // World validation passed
                }
                Err(e) => {
                    validation_success = false;
                    analysis.diagnostics.push(Diagnostic {
                        level: "error".to_string(),
                        message: format!("World validation failed: {}", e),
                        location: None,
                        suggestion: Some(
                            "Check that the specified world exists and is properly defined"
                                .to_string(),
                        ),
                        error_type: Some("world_validation".to_string()),
                        confidence: Some(0.9),
                        actionable_suggestions: vec![],
                        related_concepts: vec!["world".to_string(), "validation".to_string()],
                    });
                }
            }

            // Validate dependencies if recursive
            if recursive {
                match validate_dependencies_recursive(resolve, pkg) {
                    Ok(_) => {
                        // Dependency validation passed
                    }
                    Err(e) => {
                        validation_success = false;
                        analysis.diagnostics.push(Diagnostic {
                            level: "error".to_string(),
                            message: format!("Dependency validation failed: {}", e),
                            location: None,
                            suggestion: Some("Ensure all dependencies are available in deps/ directory and properly organized alphabetically".to_string()),
                            error_type: Some("dependency_validation".to_string()),
                            confidence: Some(0.95),
                            actionable_suggestions: vec![],
                            related_concepts: vec!["dependencies".to_string(), "deps".to_string(), "validation".to_string()],
                        });
                    }
                }
            }

            // Only mark as valid if all validation phases passed
            analysis.valid = parsing_successful && validation_success;
        }
        Err(e) => {
            analysis.diagnostics.push(create_enhanced_diagnostic(&e));
            // valid remains false
        }
    }

    Ok(analysis)
}

// Helper function for validating world selection
fn validate_world_selection(
    resolve: &Resolve,
    pkg: wit_parser::PackageId,
    world_name: Option<&str>,
) -> Result<wit_parser::WorldId> {
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

    // Create a minimal Common struct for error analysis
    let dummy_wit_path = std::path::PathBuf::from("dummy.wit");
    let dummy_context = Common {
        wit: dummy_wit_path,
        out_dir: None,
        world: None,
        check: false,
        features: Vec::new(),
        all_features: false,
    };

    // Use comprehensive error analysis
    let error_msg = e.to_string();
    let analyzed_error = analyze_dependency_error(&error_msg, &dummy_context);
    let suggestions = generate_actionable_suggestions(&analyzed_error, &dummy_context);

    // Display structured suggestions
    eprintln!("\nDiagnostic Analysis:");
    match &analyzed_error {
        DependencyResolutionError::PackageNotFound {
            package,
            context,
            searched_locations,
        } => {
            eprintln!("  - Missing Package: {}", package);
            eprintln!("  - Context: {}", context);
            eprintln!("  - Searched locations:");
            for location in searched_locations {
                eprintln!("    - {}", location);
            }
        }
        DependencyResolutionError::ParseError {
            file,
            message,
            line_number,
        } => {
            eprintln!("  - Parse Error in: {}", file);
            eprintln!("  - Message: {}", message);
            if let Some(line) = line_number {
                eprintln!("  - Line: {}", line);
            }
        }
        DependencyResolutionError::WorldNotFound {
            world,
            available_worlds,
        } => {
            eprintln!("  - Missing World: {}", world);
            if !available_worlds.is_empty() {
                eprintln!("  - Available worlds: {}", available_worlds.join(", "));
            }
        }
        DependencyResolutionError::InvalidPackageStructure { path, reason, .. } => {
            eprintln!("  - Invalid Structure: {}", path);
            eprintln!("  - Reason: {}", reason);
        }
        DependencyResolutionError::CircularDependency { chain } => {
            eprintln!("  - Circular Dependency: {}", chain.join(" -> "));
        }
        DependencyResolutionError::VersionConflict {
            package,
            required,
            found,
        } => {
            eprintln!("  - Version Conflict in: {}", package);
            eprintln!("  - Required: {}, Found: {}", required, found);
        }
        DependencyResolutionError::GenericError { category, .. } => {
            eprintln!("  - Error Category: {}", category);
        }
    }

    // Display actionable suggestions
    eprintln!("\nActionable Suggestions:");
    for (i, suggestion) in suggestions.iter().enumerate() {
        eprintln!("  {}. {}", i + 1, suggestion);
    }

    // Add personalized suggestions from usage tracking
    let personalized_count = with_usage_tracker(|tracker| {
        let error_type = match &analyzed_error {
            DependencyResolutionError::PackageNotFound { .. } => "package_not_found",
            DependencyResolutionError::ParseError { .. } => "parse_error",
            DependencyResolutionError::WorldNotFound { .. } => "world_not_found",
            _ => "general",
        };

        let personalized = tracker.get_personalized_suggestions(error_type);
        if !personalized.is_empty() {
            eprintln!("\nPersonalized Suggestions (based on your experience):");
            for (i, suggestion) in personalized.iter().enumerate() {
                eprintln!("  {}. {}", suggestions.len() + i + 1, suggestion);
            }
        }
        personalized.len()
    });

    // Fallback to original help if no suggestions were generated
    if suggestions.is_empty() && personalized_count == 0 {
        provide_general_help();
    }

    Err(e)
}

/// Create structured JSON error analysis for API consumption
fn create_json_error_analysis(e: &anyhow::Error, opts: &Common) -> serde_json::Value {
    let error_msg = e.to_string();
    let analyzed_error = analyze_dependency_error(&error_msg, opts);
    let suggestions = generate_actionable_suggestions(&analyzed_error, opts);

    let (error_type, details) = match &analyzed_error {
        DependencyResolutionError::PackageNotFound {
            package,
            context,
            searched_locations,
        } => (
            "package_not_found",
            serde_json::json!({
                "package": package,
                "context": context,
                "searched_locations": searched_locations
            }),
        ),
        DependencyResolutionError::ParseError {
            file,
            message,
            line_number,
        } => (
            "parse_error",
            serde_json::json!({
                "file": file,
                "message": message,
                "line_number": line_number
            }),
        ),
        DependencyResolutionError::WorldNotFound {
            world,
            available_worlds,
        } => (
            "world_not_found",
            serde_json::json!({
                "world": world,
                "available_worlds": available_worlds
            }),
        ),
        DependencyResolutionError::InvalidPackageStructure {
            path,
            reason,
            suggestions: struct_suggestions,
        } => (
            "invalid_package_structure",
            serde_json::json!({
                "path": path,
                "reason": reason,
                "structure_suggestions": struct_suggestions
            }),
        ),
        DependencyResolutionError::CircularDependency { chain } => (
            "circular_dependency",
            serde_json::json!({
                "dependency_chain": chain
            }),
        ),
        DependencyResolutionError::VersionConflict {
            package,
            required,
            found,
        } => (
            "version_conflict",
            serde_json::json!({
                "package": package,
                "required_version": required,
                "found_version": found
            }),
        ),
        DependencyResolutionError::GenericError { category, .. } => (
            category.as_str(),
            serde_json::json!({
                "message": error_msg
            }),
        ),
    };

    serde_json::json!({
        "valid": false,
        "error": {
            "type": error_type,
            "message": error_msg,
            "details": details
        },
        "suggestions": suggestions,
        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        "wit_bindgen_version": version()
    })
}

// Specific error help functions
#[allow(dead_code)]
fn provide_directory_resolution_help() {
    eprintln!("\nSuggestions:");
    eprintln!("  - Check that the WIT directory exists and contains .wit files");
    eprintln!("  - Check deps/ directory structure for package dependencies");
    eprintln!("  - Ensure all imported packages are in the deps/ directory");
    eprintln!("  - Check file permissions on the WIT directory");
}

#[allow(dead_code)]
fn provide_package_not_found_help(error_msg: &str) {
    eprintln!("\nSuggestions:");
    eprintln!("  - Place missing package in deps/ directory as:");
    eprintln!("    deps/missing-package/  (for package directories)");
    eprintln!("    deps/missing-package.wit  (for single WIT files)");
    eprintln!("  - Ensure packages are in alphabetical order in deps/");

    // Try to extract package name from error for more specific help
    if let Some(start) = error_msg.find("package '") {
        if let Some(end) = error_msg[start + 9..].find("'") {
            let package_name = &error_msg[start + 9..start + 9 + end];
            eprintln!("  - For package '{}', try:", package_name);
            eprintln!("    - Check if it exists in your deps/ directory");
            eprintln!("    - Verify the package name matches exactly");
        }
    }
}

#[allow(dead_code)]
fn provide_world_not_found_help() {
    eprintln!("\nSuggestions:");
    eprintln!("  - Check that the world name matches exactly (case-sensitive)");
    eprintln!("  - Use `wit-bindgen validate --show-tree` to see available worlds");
    eprintln!("  - Try running without specifying a world to use the default");
}

#[allow(dead_code)]
fn provide_syntax_error_help() {
    eprintln!("\nSuggestions:");
    eprintln!("  - Check WIT syntax - look for missing semicolons, brackets, or keywords");
    eprintln!("  - Verify package declarations start with 'package namespace:name'");
    eprintln!("  - Ensure interface and world definitions are properly closed");
    eprintln!("  - Check for typos in WIT keywords (interface, world, type, etc.)");
}

#[allow(dead_code)]
fn provide_interface_error_help() {
    eprintln!("\nSuggestions:");
    eprintln!("  - Check that interface names are defined before being used");
    eprintln!("  - Verify import statements include the correct package namespace");
    eprintln!("  - Ensure interface definitions are in the right package");
}

#[allow(dead_code)]
fn provide_type_error_help() {
    eprintln!("\nSuggestions:");
    eprintln!("  - Define custom types before using them in functions");
    eprintln!("  - Check spelling of type names (case-sensitive)");
    eprintln!("  - Import types from other packages if needed");
}

fn provide_general_help() {
    eprintln!("\nGeneral troubleshooting:");
    eprintln!("  - Run with `wit-bindgen validate --show-tree` for more details");
    eprintln!("  - Check the wit-bindgen documentation for syntax examples");
    eprintln!("  - Verify all .wit files have valid syntax");
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

fn validate_dependencies_recursive(
    resolve: &Resolve,
    root_pkg: wit_parser::PackageId,
) -> Result<()> {
    let mut validation_errors = Vec::new();

    // Track which packages we've seen
    let mut validated_packages = std::collections::HashSet::new();

    // Validate the root package and all its dependencies
    validate_package_dependencies(
        resolve,
        root_pkg,
        &mut validated_packages,
        &mut validation_errors,
    )?;

    if !validation_errors.is_empty() {
        let error_msg = format!(
            "Dependency validation failed with {} error(s): {}",
            validation_errors.len(),
            validation_errors.join("; ")
        );
        bail!(error_msg);
    }

    Ok(())
}

fn validate_package_dependencies(
    resolve: &Resolve,
    pkg_id: wit_parser::PackageId,
    validated: &mut std::collections::HashSet<wit_parser::PackageId>,
    errors: &mut Vec<String>,
) -> Result<()> {
    if validated.contains(&pkg_id) {
        return Ok(()); // Already validated
    }

    validated.insert(pkg_id);
    let package = &resolve.packages[pkg_id];

    // Validate that all interfaces in this package are properly resolved
    for (_, interface_id) in package.interfaces.iter() {
        let interface = &resolve.interfaces[*interface_id];

        // Check that all types used in this interface are resolvable
        for (_, func) in interface.functions.iter() {
            // Validate parameter types
            for (_, param_type) in func.params.iter() {
                if let Err(e) = validate_type_resolution(resolve, param_type) {
                    errors.push(format!(
                        "Package {}:{}, function {}: parameter type validation failed: {}",
                        package.name.namespace, package.name.name, func.name, e
                    ));
                }
            }

            // Validate return types
            if let Some(return_type) = &func.result {
                if let Err(e) = validate_type_resolution(resolve, return_type) {
                    errors.push(format!(
                        "Package {}:{}, function {}: return type validation failed: {}",
                        package.name.namespace, package.name.name, func.name, e
                    ));
                }
            }
        }
    }

    // Recursively validate dependencies (packages that this package imports from)
    for (_, other_pkg) in resolve.packages.iter() {
        if other_pkg.name.namespace != package.name.namespace
            || other_pkg.name.name != package.name.name
        {
            // This is a different package - validate it if it's referenced
            // Skip recursive validation for now - this needs proper dependency graph analysis
            // validate_package_dependencies(resolve, other_pkg_id, validated, errors)?;
        }
    }

    Ok(())
}

fn validate_type_resolution(resolve: &Resolve, wit_type: &wit_parser::Type) -> Result<()> {
    match wit_type {
        wit_parser::Type::Id(type_id) => {
            // Check that the type ID can be resolved
            if resolve.types.get(*type_id).is_none() {
                bail!("Type ID {:?} cannot be resolved", type_id);
            }
            Ok(())
        }
        // For primitive types, validation always succeeds
        _ => Ok(()),
    }
}

fn print_dependency_tree(resolve: &Resolve, root_pkg: wit_parser::PackageId) {
    eprintln!("\nDependency tree:");
    let root_package = &resolve.packages[root_pkg];
    eprintln!(
        "root: {}:{}",
        root_package.name.namespace, root_package.name.name
    );

    // List all packages that were resolved
    for (_pkg_id, package) in resolve.packages.iter() {
        if package.name.namespace != root_package.name.namespace
            || package.name.name != root_package.name.name
        {
            eprintln!("  dep: {}:{}", package.name.namespace, package.name.name);
        }
    }
}

fn print_next_steps(output_dir: &PathBuf, wit_path: &PathBuf) {
    eprintln!("\nNext steps:");
    eprintln!("  1. cd {}", output_dir.display());
    eprintln!("  2. Implement the TODO functions in src/lib.rs");
    eprintln!("  3. Build with: cargo build --target wasm32-wasip2");
    eprintln!(
        "  4. Test with: wit-bindgen validate {}",
        wit_path.display()
    );
}

fn generate_scaffolding(
    opts: &Common,
    output_dir: &PathBuf,
    with_cargo: bool,
    name: Option<String>,
) -> Result<()> {
    generate_scaffolding_with_format(
        opts,
        output_dir,
        with_cargo,
        name,
        OutputFormat::Human,
        false,
    )
}

fn generate_scaffolding_with_format(
    opts: &Common,
    output_dir: &PathBuf,
    with_cargo: bool,
    name: Option<String>,
    format: OutputFormat,
    report: bool,
) -> Result<()> {
    // Use the existing WorldGenerator infrastructure
    let component_name = name.unwrap_or_else(|| "component".to_string());
    let generator = ScaffoldGenerator {
        output_dir: output_dir.clone(),
        with_cargo,
        component_name: component_name.clone(),
    };

    let mut files = Files::default();
    gen_world(Box::new(generator), opts, &mut files)?;

    // Check for existing files before writing
    let mut existing_files = Vec::new();
    for (name, _) in files.iter() {
        let dst = output_dir.join(name);
        if dst.exists() {
            existing_files.push(dst.display().to_string());
        }
    }

    // If any files exist, warn the user and abort
    if !existing_files.is_empty() {
        match format {
            OutputFormat::Human => {
                eprintln!("Error: The following files already exist and would be overwritten:");
                for file in &existing_files {
                    eprintln!("  - {}", file);
                }
                eprintln!("\nTo proceed, please either:");
                eprintln!("  1. Remove the existing files");
                eprintln!("  2. Choose a different output directory");
                eprintln!("  3. Use --force to overwrite (not yet implemented)");
            }
            OutputFormat::Json => {
                let error = serde_json::json!({
                    "error": "Files already exist",
                    "existing_files": existing_files,
                    "suggestions": [
                        "Remove the existing files",
                        "Choose a different output directory"
                    ]
                });
                println!("{}", serde_json::to_string_pretty(&error)?);
            }
        }
        bail!("Refusing to overwrite existing files");
    }

    // Write files to disk
    let mut generated_files = Vec::new();
    for (name, contents) in files.iter() {
        let dst = output_dir.join(name);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {:?}", parent))?;
        }
        std::fs::write(&dst, contents).with_context(|| format!("failed to write {:?}", dst))?;
        generated_files.push(name.to_string());
    }

    match format {
        OutputFormat::Human => {
            print_next_steps(output_dir, &opts.wit);
        }
        OutputFormat::Json => {
            if report {
                let report = ScaffoldReport {
                    generated_files,
                    module_structure: vec![ModuleInfo {
                        path: "src/lib.rs".to_string(),
                        module_type: "implementation".to_string(),
                    }],
                    required_implementations: vec!["exports::*::Guest traits".to_string()],
                };
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
        }
    }

    Ok(())
}

fn generate_cargo_toml(component_name: &str, world_name: &str) -> Result<String> {
    // Use the same version as the CLI tool, but only the semantic version part
    let wit_bindgen_version = version();
    let wit_bindgen_version = wit_bindgen_version
        .split_whitespace()
        .next()
        .unwrap_or("0.43.0");

    Ok(format!(
        r#"[package]
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
"#,
        component_name, world_name, wit_bindgen_version
    ))
}

// Updated function to work with WorldId directly (for Files infrastructure)
fn generate_lib_rs_from_world(
    resolve: &Resolve,
    world_id: WorldId,
    world_name: &str,
) -> Result<String> {
    let world = &resolve.worlds[world_id];

    let mut content = String::new();
    content.push_str(&format!(
        r#"// Generated component implementation for world '{}'
// TODO: Implement the functions marked with TODO comments

wit_bindgen::generate!({{
    world: "{}",
    path: "wit/",
    // Uncomment to see generated module paths:
    // show_module_paths: true,
}});

struct Component;

"#,
        world_name, world_name
    ));

    // Generate export implementations
    for (key, item) in world.exports.iter() {
        match item {
            wit_parser::WorldItem::Interface { id, .. } => {
                let interface = &resolve.interfaces[*id];
                let module_path = compute_export_module_path(resolve, key);

                content.push_str(&format!("impl {}::Guest for Component {{\n", module_path));

                // Generate function stubs for interface
                for (_name, func) in interface.functions.iter() {
                    content.push_str(&generate_function_stub(func, resolve));
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
                content.push_str(&generate_function_stub(func, resolve));
                content.push_str("}\n\n");
            }
            _ => {}
        }
    }

    content.push_str("export!(Component);\n");
    Ok(content)
}

// Helper function to format WIT function signature
fn format_wit_function(func: &wit_parser::Function) -> String {
    let params = func
        .params
        .iter()
        .map(|(name, ty)| format!("{}: {}", name, format_wit_type(ty)))
        .collect::<Vec<_>>()
        .join(", ");

    let result = match &func.result {
        Some(ty) => format!(" -> {}", format_wit_type(ty)),
        None => String::new(),
    };

    format!("{}: func({}){}", func.name, params, result)
}

// Helper function to format Rust function signature
fn format_rust_function(func: &wit_parser::Function, resolve: &Resolve) -> String {
    let params = func
        .params
        .iter()
        .map(|(name, ty)| {
            format!(
                "{}: {}",
                name.replace('-', "_"),
                map_wit_type_to_rust_with_context(ty, resolve)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    let result = match &func.result {
        Some(ty) => format!(" -> {}", map_wit_type_to_rust_with_context(ty, resolve)),
        None => String::new(),
    };

    format!("fn {}({}){}", func.name.replace('-', "_"), params, result)
}

// Helper function to format WIT types
fn format_wit_type(ty: &wit_parser::Type) -> String {
    match ty {
        wit_parser::Type::Bool => "bool".to_string(),
        wit_parser::Type::U8 => "u8".to_string(),
        wit_parser::Type::U16 => "u16".to_string(),
        wit_parser::Type::U32 => "u32".to_string(),
        wit_parser::Type::U64 => "u64".to_string(),
        wit_parser::Type::S8 => "s8".to_string(),
        wit_parser::Type::S16 => "s16".to_string(),
        wit_parser::Type::S32 => "s32".to_string(),
        wit_parser::Type::S64 => "s64".to_string(),
        wit_parser::Type::F32 => "f32".to_string(),
        wit_parser::Type::F64 => "f64".to_string(),
        wit_parser::Type::Char => "char".to_string(),
        wit_parser::Type::String => "string".to_string(),
        wit_parser::Type::Id(_) => "type-id".to_string(), // Simplified for now
        wit_parser::Type::ErrorContext => "error-context".to_string(),
    }
}

// Helper function to create implementation guide
fn create_implementation_guide(resolve: &Resolve, world_id: WorldId) -> ImplementationGuide {
    let world = &resolve.worlds[world_id];
    let mut required_traits = Vec::new();

    for (key, item) in world.exports.iter() {
        if let wit_parser::WorldItem::Interface { .. } = item {
            let module_path = compute_export_module_path(resolve, key);
            required_traits.push(format!("{}::Guest", module_path));
        }
    }

    ImplementationGuide {
        required_traits,
        boilerplate: BoilerplateCode {
            struct_definition: "struct Component;".to_string(),
            export_macro: "export!(Component);".to_string(),
            generate_macro: format!(
                "wit_bindgen::generate!({{\n    world: \"{}\",\n    path: \"wit/\",\n}});",
                world.name
            ),
        },
        examples: std::collections::HashMap::new(),
    }
}

// Helper function to create type mappings
fn create_type_mappings() -> std::collections::HashMap<String, String> {
    let mut mappings = std::collections::HashMap::new();
    mappings.insert("string".to_string(), "String".to_string());
    mappings.insert("bool".to_string(), "bool".to_string());
    mappings.insert("u8".to_string(), "u8".to_string());
    mappings.insert("u16".to_string(), "u16".to_string());
    mappings.insert("u32".to_string(), "u32".to_string());
    mappings.insert("u64".to_string(), "u64".to_string());
    mappings.insert("s8".to_string(), "i8".to_string());
    mappings.insert("s16".to_string(), "i16".to_string());
    mappings.insert("s32".to_string(), "i32".to_string());
    mappings.insert("s64".to_string(), "i64".to_string());
    mappings.insert("f32".to_string(), "f32".to_string());
    mappings.insert("f64".to_string(), "f64".to_string());
    mappings.insert("char".to_string(), "char".to_string());
    mappings.insert("list<u8>".to_string(), "Vec<u8>".to_string());
    mappings.insert("option<T>".to_string(), "Option<T>".to_string());
    mappings.insert("result<T, E>".to_string(), "Result<T, E>".to_string());
    mappings
}

// Helper function to extract suggestion from error
fn extract_suggestion_from_error(err: &anyhow::Error) -> Option<String> {
    let err_str = err.to_string();

    if err_str.contains("package not found") {
        Some("Place the missing package in the deps/ directory (deps/package-name/ or deps/package-name.wit)".to_string())
    } else if err_str.contains("world not found") {
        Some("Check the world name spelling or use --show-tree to see available worlds".to_string())
    } else if err_str.contains("failed to resolve directory") {
        Some("Ensure the WIT directory exists and contains valid .wit files".to_string())
    } else {
        None
    }
}

/// Create enhanced diagnostic with detailed analysis
fn create_enhanced_diagnostic(err: &anyhow::Error) -> Diagnostic {
    let err_str = err.to_string();

    // Classify error type and generate suggestions
    let (error_type, suggestions, confidence) = classify_error_and_generate_suggestions(&err_str);

    // Extract related concepts based on error type
    let related_concepts = extract_related_concepts(&error_type);

    Diagnostic {
        level: "error".to_string(),
        message: err_str.clone(),
        location: None,
        suggestion: extract_suggestion_from_error(err),
        error_type: Some(error_type.clone()),
        confidence: Some(confidence),
        actionable_suggestions: suggestions,
        related_concepts,
    }
}

/// Classify error type and generate actionable suggestions
fn classify_error_and_generate_suggestions(
    err_str: &str,
) -> (String, Vec<ActionableSuggestion>, f32) {
    if err_str.contains("failed to resolve directory while parsing WIT") {
        let suggestions = vec![
            ActionableSuggestion {
                action: "Validate WIT directory structure".to_string(),
                command: Some("ls -la".to_string()),
                priority: "critical".to_string(),
                estimated_success_rate: 0.8,
                explanation: "Check if WIT directory exists and contains .wit files".to_string(),
            },
            ActionableSuggestion {
                action: "Check dependencies configuration".to_string(),
                command: Some("wit-bindgen deps --check-deps".to_string()),
                priority: "high".to_string(),
                estimated_success_rate: 0.9,
                explanation: "Verify all dependencies are properly configured".to_string(),
            },
            ActionableSuggestion {
                action: "Generate dependency template".to_string(),
                command: Some("wit-bindgen deps --generate".to_string()),
                priority: "medium".to_string(),
                estimated_success_rate: 0.7,
                explanation: "Create proper deps/ directory structure for missing dependencies"
                    .to_string(),
            },
        ];
        ("dependency_resolution".to_string(), suggestions, 0.95)
    } else if err_str.contains("package not found") || err_str.contains("unresolved import") {
        let package_name = extract_package_name_from_error(err_str);
        let mut suggestions = vec![
            ActionableSuggestion {
                action: "Add missing package to deps/ directory".to_string(),
                command: package_name
                    .as_ref()
                    .map(|p| format!("wit-bindgen deps --add {}", p)),
                priority: "critical".to_string(),
                estimated_success_rate: 0.85,
                explanation: "Add the missing package dependency to your project".to_string(),
            },
            ActionableSuggestion {
                action: "Verify package name spelling".to_string(),
                command: None,
                priority: "high".to_string(),
                estimated_success_rate: 0.6,
                explanation: "Check that import names match package declarations exactly"
                    .to_string(),
            },
        ];

        if package_name.is_some() {
            suggestions[0].action =
                format!("Add package '{}' to deps/ directory", package_name.unwrap());
        }

        ("package_not_found".to_string(), suggestions, 0.9)
    } else if err_str.contains("world not found") {
        let suggestions = vec![
            ActionableSuggestion {
                action: "List available worlds".to_string(),
                command: Some("wit-bindgen validate --show-tree".to_string()),
                priority: "critical".to_string(),
                estimated_success_rate: 0.9,
                explanation: "Display all worlds defined in your WIT files".to_string(),
            },
            ActionableSuggestion {
                action: "Check world name case sensitivity".to_string(),
                command: None,
                priority: "high".to_string(),
                estimated_success_rate: 0.7,
                explanation: "World names are case-sensitive and must match exactly".to_string(),
            },
            ActionableSuggestion {
                action: "Use default world selection".to_string(),
                command: None,
                priority: "medium".to_string(),
                estimated_success_rate: 0.6,
                explanation: "Try omitting the world parameter to use the default world"
                    .to_string(),
            },
        ];
        ("world_not_found".to_string(), suggestions, 0.9)
    } else if err_str.contains("syntax error") || err_str.contains("expected") {
        let suggestions = vec![
            ActionableSuggestion {
                action: "Validate WIT syntax".to_string(),
                command: Some("wit-bindgen validate".to_string()),
                priority: "critical".to_string(),
                estimated_success_rate: 0.8,
                explanation: "Check for syntax errors in your WIT files".to_string(),
            },
            ActionableSuggestion {
                action: "Review WIT syntax documentation".to_string(),
                command: None,
                priority: "medium".to_string(),
                estimated_success_rate: 0.6,
                explanation: "Consult WIT syntax reference for correct formatting".to_string(),
            },
        ];
        ("wit_syntax".to_string(), suggestions, 0.85)
    } else {
        let suggestions = vec![ActionableSuggestion {
            action: "Run comprehensive validation".to_string(),
            command: Some("wit-bindgen validate --analyze".to_string()),
            priority: "high".to_string(),
            estimated_success_rate: 0.6,
            explanation: "Perform detailed analysis to identify potential issues".to_string(),
        }];
        ("unknown".to_string(), suggestions, 0.3)
    }
}

/// Extract related concepts based on error type
fn extract_related_concepts(error_type: &str) -> Vec<String> {
    match error_type {
        "dependency_resolution" => vec![
            "WIT packages".to_string(),
            "deps/ directory structure".to_string(),
            "dependency management".to_string(),
            "package imports".to_string(),
        ],
        "package_not_found" => vec![
            "WIT imports".to_string(),
            "package namespaces".to_string(),
            "dependency resolution".to_string(),
            "package declarations".to_string(),
        ],
        "world_not_found" => vec![
            "WIT worlds".to_string(),
            "world selection".to_string(),
            "component interfaces".to_string(),
        ],
        "wit_syntax" => vec![
            "WIT language syntax".to_string(),
            "interface definitions".to_string(),
            "type declarations".to_string(),
        ],
        _ => vec!["WIT debugging".to_string()],
    }
}

/// Extract package name from error messages
fn extract_package_name_from_error(error_str: &str) -> Option<String> {
    if let Some(start) = error_str.find("package '") {
        if let Some(end) = error_str[start + 9..].find("'") {
            return Some(error_str[start + 9..start + 9 + end].to_string());
        }
    }

    if let Some(start) = error_str.find("package `") {
        if let Some(end) = error_str[start + 9..].find("`") {
            return Some(error_str[start + 9..start + 9 + end].to_string());
        }
    }

    None
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
                format!(
                    "exports::{}::{}::{}",
                    pkg.name.namespace.replace('-', "_"),
                    pkg.name.name.replace('-', "_"),
                    interface.name.as_ref().unwrap().replace('-', "_")
                )
            } else {
                "exports::interface".to_string()
            }
        }
    }
}

fn generate_function_stub(func: &wit_parser::Function, resolve: &Resolve) -> String {
    let mut stub = String::new();

    // Generate function signature with proper types
    stub.push_str(&format!("    fn {}(", func.name.replace('-', "_")));

    // Generate parameters with proper type mapping
    for (i, (name, ty)) in func.params.iter().enumerate() {
        if i > 0 {
            stub.push_str(", ");
        }
        stub.push_str(&format!(
            "{}: {},",
            name.replace('-', "_"),
            map_wit_type_to_rust_with_context(ty, resolve)
        ));
    }

    stub.push_str(")");

    // Generate return type with proper mapping
    if let Some(result_ty) = &func.result {
        stub.push_str(&format!(
            " -> {}",
            map_wit_type_to_rust_with_context(result_ty, resolve)
        ));
    }

    stub.push_str(" {\n");
    stub.push_str(&format!("        // TODO: Implement {}\n", func.name));
    stub.push_str("        todo!()\n");
    stub.push_str("    }\n\n");

    stub
}

// Enhanced version that can resolve actual type names with context
fn map_wit_type_to_rust_with_context(ty: &wit_parser::Type, resolve: &Resolve) -> String {
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
            let type_def = &resolve.types[*id];

            // If the type has a name, use it with proper Rust formatting
            if let Some(name) = &type_def.name {
                let rust_name = name.to_upper_camel_case();

                // Determine the module path based on the package
                match type_def.owner {
                    wit_parser::TypeOwner::World(_) => rust_name,
                    wit_parser::TypeOwner::Interface(interface_id) => {
                        let interface = &resolve.interfaces[interface_id];
                        if let Some(pkg_id) = interface.package {
                            let pkg = &resolve.packages[pkg_id];
                            let default_name = "unnamed".to_string();
                            let interface_name = interface.name.as_ref().unwrap_or(&default_name);
                            format!(
                                "exports::{}::{}::{}",
                                pkg.name.namespace.replace('-', "_"),
                                pkg.name.name.replace('-', "_"),
                                interface_name.replace('-', "_")
                            )
                        } else if let Some(interface_name) = &interface.name {
                            format!("exports::{}", interface_name.replace('-', "_"))
                        } else {
                            rust_name
                        }
                    }
                    wit_parser::TypeOwner::None => rust_name,
                }
            } else {
                // For anonymous types, try to infer based on the type definition
                match &type_def.kind {
                    wit_parser::TypeDefKind::Option(inner) => {
                        format!(
                            "Option<{}>",
                            map_wit_type_to_rust_with_context(inner, resolve)
                        )
                    }
                    wit_parser::TypeDefKind::Result(result) => {
                        let ok_type = result
                            .ok
                            .as_ref()
                            .map(|t| map_wit_type_to_rust_with_context(t, resolve))
                            .unwrap_or_else(|| "()".to_string());
                        let err_type = result
                            .err
                            .as_ref()
                            .map(|t| map_wit_type_to_rust_with_context(t, resolve))
                            .unwrap_or_else(|| "String".to_string());
                        format!("Result<{}, {}>", ok_type, err_type)
                    }
                    wit_parser::TypeDefKind::List(inner) => {
                        format!("Vec<{}>", map_wit_type_to_rust_with_context(inner, resolve))
                    }
                    wit_parser::TypeDefKind::Record(_) => "/* Record */".to_string(),
                    wit_parser::TypeDefKind::Variant(_) => "/* Variant */".to_string(),
                    wit_parser::TypeDefKind::Enum(_) => "/* Enum */".to_string(),
                    wit_parser::TypeDefKind::Tuple(tuple) => {
                        let types = tuple
                            .types
                            .iter()
                            .map(|t| map_wit_type_to_rust_with_context(t, resolve))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("({})", types)
                    }
                    wit_parser::TypeDefKind::Resource => "/* Resource */".to_string(),
                    wit_parser::TypeDefKind::Handle(_) => "/* Handle */".to_string(),
                    wit_parser::TypeDefKind::Flags(_) => "/* Flags */".to_string(),
                    _ => format!("/* UnknownType{} */", id.index()),
                }
            }
        }
        wit_parser::Type::ErrorContext => "/* ErrorContext */".to_string(),
    }
}

fn generate_resource_stub(name: &str) -> String {
    let resource_name = name.replace('-', "_");
    let type_name = resource_name.to_uppercase();

    format!(
        r#"    type {} = (); // TODO: Define your resource type
    
    fn [new-{}]() -> Self::{} {{
        // TODO: Implement resource constructor
        todo!()
    }}
    
    fn [drop](_rep: Self::{}) {{
        // TODO: Implement resource destructor
        todo!()
    }}
    
"#,
        type_name, resource_name, type_name, type_name
    )
}

fn generate_readme(component_name: &str, world_name: &str) -> Result<String> {
    Ok(format!(
        r#"# {} Component

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
"#,
        component_name, world_name, component_name
    ))
}

fn run_interactive_mode(opts: &Common, guided: bool) -> Result<()> {
    eprintln!("Welcome to wit-bindgen Interactive Mode!");
    println!();

    if guided {
        eprintln!(
            "This guided mode will walk you through creating a WebAssembly component step-by-step."
        );
        println!();
    }

    // Step 1: Validate WIT files
    eprintln!("Step 1: Validating WIT dependencies...");
    let validation_result =
        validate_wit_dependencies(opts, false, false, OutputFormat::Human, false, false);

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

    let (pkg, _files) = resolve
        .push_path(&opts.wit)
        .with_context(|| "Failed to parse WIT files")?;

    let world_id = resolve
        .select_world(pkg, opts.world.as_deref())
        .with_context(|| "Failed to select world")?;

    let world = &resolve.worlds[world_id];

    eprintln!("Step 2: Analyzing WIT structure...");
    print_world_structure(&resolve, world_id);

    if guided {
        println!();
        eprintln!("Your component will need to:");

        if !world.exports.is_empty() {
            eprintln!("   - Implement {} export interface(s)", world.exports.len());
            for (key, _item) in world.exports.iter() {
                eprintln!("     - {}", resolve.name_world_key(key));
            }
        }

        if !world.imports.is_empty() {
            eprintln!("   - Use {} imported interface(s)", world.imports.len());
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
            let component_name =
                prompt_string("Component name", Some(&world.name.replace('-', "_")))?;
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
                    println!(
                        "  export '{}' -> impl {}::Guest",
                        resolve.name_world_key(key),
                        module_path
                    );
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
        _ => unreachable!(),
    }

    if guided {
        println!();
        println!("You're all set! Here are some helpful next steps:");
        println!("  - Read the generated README.md for detailed instructions");
        println!("  - Use `wit-bindgen validate` to check your WIT files anytime");
        println!("  - Join the WebAssembly community for support and questions");
        println!("  - Check out the component model documentation at component-model.bytecodealliance.org");
    }

    Ok(())
}

fn prompt_yes_no(question: &str) -> Result<bool> {
    loop {
        print!("{} [y/N]: ", question);
        io::stdout().flush()?;

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {}
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
            Ok(_) => {}
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

// Analyze command for tooling
fn run_analyze_command(opts: &Common, templates: bool, format: OutputFormat) -> Result<()> {
    let mut resolve = setup_resolve_with_features(opts);

    // Always include comprehensive analysis for tooling
    match analyze_wit_to_json(&mut resolve, opts, true, true, true) {
        Ok(analysis) => {
            match format {
                OutputFormat::Json => {
                    // For automation, always output structured JSON
                    println!("{}", serde_json::to_string_pretty(&analysis)?);
                }
                OutputFormat::Human => {
                    // Human-readable summary
                    eprintln!("WIT Analysis Summary");
                    eprintln!("===================");
                    eprintln!("Valid: {}", analysis.valid);
                    eprintln!("Worlds: {}", analysis.worlds.len());
                    eprintln!("Dependencies: {}", analysis.dependencies.len());
                    eprintln!("Diagnostics: {}", analysis.diagnostics.len());

                    if templates && analysis.implementation_guide.is_some() {
                        eprintln!("\nImplementation Guide:");
                        if let Some(guide) = &analysis.implementation_guide {
                            eprintln!("Required traits:");
                            for trait_name in &guide.required_traits {
                                eprintln!("  - {}", trait_name);
                            }
                            eprintln!("\nBoilerplate:");
                            eprintln!("{}", guide.boilerplate.generate_macro);
                            eprintln!("{}", guide.boilerplate.struct_definition);
                            eprintln!("{}", guide.boilerplate.export_macro);
                        }
                    }
                }
            }
        }
        Err(e) => match format {
            OutputFormat::Json => {
                let error_analysis = create_json_error_analysis(&e, opts);
                println!("{}", serde_json::to_string_pretty(&error_analysis)?);
                return Err(e);
            }
            OutputFormat::Human => {
                return handle_validation_error(e);
            }
        },
    }

    Ok(())
}

fn pause_for_user(message: &str) -> Result<()> {
    print!("{}", message);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(())
}

// Dependency management command
fn run_deps_command(
    opts: &Common,
    check: bool,
    generate: bool,
    add: Option<String>,
    from: Option<String>,
    fix: bool,
    sync_check: bool,
    order_fix: bool,
    format: OutputFormat,
) -> Result<()> {
    if check {
        return check_dependencies(opts, format);
    }

    if generate {
        return generate_deps_toml(opts, format);
    }

    if let Some(dependency) = add {
        return add_dependency(opts, &dependency, from.as_deref(), format);
    }

    if fix {
        return fix_dependencies(opts, format);
    }

    if sync_check {
        return check_dependency_sync(opts, format);
    }

    if order_fix {
        return fix_alphabetical_ordering(opts, format);
    }

    // Default: analyze dependencies
    analyze_dependencies(opts, format)
}

fn check_dependencies(opts: &Common, format: OutputFormat) -> Result<()> {
    let mut resolve = setup_resolve_with_features(opts);
    let mut missing_deps = Vec::new();
    let mut found_deps = Vec::new();

    // Try to parse WIT files and track dependencies
    match resolve.push_path(&opts.wit) {
        Ok((_pkg_id, _)) => {
            // Collect all referenced packages
            for (_id, package) in resolve.packages.iter() {
                let dep_info = DependencyInfo {
                    package: format!("{}:{}", package.name.namespace, package.name.name),
                    namespace: package.name.namespace.clone(),
                    name: package.name.name.clone(),
                };
                found_deps.push(dep_info);
            }

            match format {
                OutputFormat::Human => {
                    eprintln!("Dependencies check passed!");
                    eprintln!("Found {} packages:", found_deps.len());
                    for dep in &found_deps {
                        eprintln!("  ✓ {}", dep.package);
                    }
                }
                OutputFormat::Json => {
                    let result = serde_json::json!({
                        "success": true,
                        "found_dependencies": found_deps,
                        "missing_dependencies": missing_deps,
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
        }
        Err(e) => {
            // Parse error to find missing dependencies
            let error_str = e.to_string();
            if error_str.contains("package") && error_str.contains("not found") {
                // Try to extract package name from error
                if let Some(package_name) = extract_package_from_error(&error_str) {
                    missing_deps.push(package_name);
                }
            }

            match format {
                OutputFormat::Human => {
                    eprintln!("error: Dependencies check failed!");
                    eprintln!("{}", e);
                    eprintln!("\nMissing dependencies:");
                    for dep in &missing_deps {
                        eprintln!("  ✗ {}", dep);
                    }
                    eprintln!("\nTo fix:");
                    eprintln!("  1. Run: wit-bindgen deps --generate");
                    eprintln!("  2. Or add dependencies manually to deps/ directory");
                }
                OutputFormat::Json => {
                    let result = serde_json::json!({
                        "success": false,
                        "error": e.to_string(),
                        "found_dependencies": found_deps,
                        "missing_dependencies": missing_deps,
                        "suggestions": [
                            "Run 'wit-bindgen deps --generate' to create deps/ directory structure",
                            "Add missing packages to deps/ directory"
                        ]
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }

            return Err(e);
        }
    }

    Ok(())
}

fn generate_deps_toml(opts: &Common, format: OutputFormat) -> Result<()> {
    let base_dir = opts.wit.parent().unwrap_or(&opts.wit);
    let deps_dir = base_dir.join("deps");

    // Check if deps/ directory already exists
    if deps_dir.exists() && deps_dir.is_dir() {
        match format {
            OutputFormat::Human => {
                eprintln!("deps/ directory already exists at: {}", deps_dir.display());
                eprintln!("Use --add to add specific dependencies to it.");
            }
            OutputFormat::Json => {
                let result = serde_json::json!({
                    "generated": false,
                    "reason": "deps/ directory already exists",
                    "path": deps_dir.display().to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        return Ok(());
    }

    // Create deps/ directory structure
    std::fs::create_dir_all(&deps_dir)?;

    // Create a README explaining the structure
    let readme_content = r#"# WIT Package Dependencies Directory

This directory contains external WIT packages that your component depends on.
wit-parser automatically discovers dependencies in this directory.

## Structure

Place dependencies as:
- `package-name/` - Directory containing WIT package files
- `package-name.wit` - Single WIT file packages

## Important Notes

1. **Alphabetical Order**: wit-parser processes dependencies alphabetically by filename
2. **No Configuration File**: Unlike other tools, wit-bindgen uses directory scanning, not configuration files
3. **Automatic Discovery**: Just place packages here and they'll be found automatically

## Examples

```
deps/
├── http-types/           # Package directory
│   ├── http.wit
│   └── types.wit
├── logging.wit           # Single file package
└── wasi-http/           # Another package directory
    └── proxy.wit
```

## Adding Dependencies

Use `wit-bindgen deps --add namespace:name` to add dependencies automatically.
"#;

    let readme_path = deps_dir.join("README.md");
    std::fs::write(&readme_path, readme_content)?;

    match format {
        OutputFormat::Human => {
            eprintln!("Created deps/ directory at: {}", deps_dir.display());
            eprintln!("Added README.md with usage instructions.");
            eprintln!("\nNext steps:");
            eprintln!("  1. Add WIT packages to deps/ directory");
            eprintln!("  2. Ensure packages are in alphabetical order");
            eprintln!("  3. Run 'wit-bindgen deps --check' to verify");
        }
        OutputFormat::Json => {
            let result = serde_json::json!({
                "generated": true,
                "path": deps_dir.display().to_string(),
                "readme_created": readme_path.display().to_string(),
                "next_steps": [
                    "Add WIT packages to deps/ directory",
                    "Ensure packages are in alphabetical order",
                    "Run 'wit-bindgen deps --check' to verify"
                ]
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}

fn add_dependency(
    opts: &Common,
    dependency: &str,
    from_path: Option<&str>,
    format: OutputFormat,
) -> Result<()> {
    // Parse dependency format: namespace:name or namespace:name@version
    let parts: Vec<&str> = dependency.split(':').collect();
    if parts.len() != 2 {
        bail!("Invalid dependency format. Use 'namespace:name' or 'namespace:name@version'");
    }

    let namespace = parts[0];
    let name_version: Vec<&str> = parts[1].split('@').collect();
    let name = name_version[0];
    let _version = name_version.get(1);

    let base_dir = opts.wit.parent().unwrap_or(&opts.wit);
    let deps_dir = base_dir.join("deps");

    // Ensure deps/ directory exists
    std::fs::create_dir_all(&deps_dir)?;

    // Create directory structure for the dependency
    let package_dir_name = format!("{}-{}", namespace, name);
    let package_dir = deps_dir.join(&package_dir_name);

    if package_dir.exists() {
        match format {
            OutputFormat::Human => {
                eprintln!(
                    "Dependency directory already exists: {}",
                    package_dir.display()
                );
                eprintln!(
                    "Add WIT files to this directory manually or use deps --fix to auto-discover."
                );
            }
            OutputFormat::Json => {
                let result = serde_json::json!({
                    "added": false,
                    "reason": "directory already exists",
                    "path": package_dir.display().to_string()
                });
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        return Ok(());
    }

    // Handle copying from local file/directory if --from is specified
    if let Some(source_path) = from_path {
        let source = std::path::Path::new(source_path);

        if !source.exists() {
            match format {
                OutputFormat::Human => {
                    eprintln!("Error: Source path does not exist: {}", source.display());
                }
                OutputFormat::Json => {
                    let result = serde_json::json!({
                        "added": false,
                        "reason": "Source path does not exist",
                        "source_path": source.display().to_string()
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            return Ok(());
        }

        if source.is_file() {
            // Copy single file
            if source.extension().map(|s| s == "wit").unwrap_or(false) {
                // Copy as single WIT file: deps/package-name.wit
                let target_file = deps_dir.join(format!("{}.wit", package_dir_name));
                std::fs::copy(source, &target_file)?;

                match format {
                    OutputFormat::Human => {
                        eprintln!(
                            "📄 Copied WIT file: {} -> {}",
                            source.display(),
                            target_file.display()
                        );
                    }
                    OutputFormat::Json => {
                        let result = serde_json::json!({
                            "added": true,
                            "type": "file",
                            "source": source.display().to_string(),
                            "target": target_file.display().to_string()
                        });
                        println!("{}", serde_json::to_string_pretty(&result)?);
                    }
                }
            } else {
                // Copy as directory with the file inside
                std::fs::create_dir_all(&package_dir)?;
                let target_file = package_dir.join(source.file_name().unwrap());
                std::fs::copy(source, &target_file)?;

                match format {
                    OutputFormat::Human => {
                        eprintln!(
                            "Copied file to package directory: {} -> {}",
                            source.display(),
                            target_file.display()
                        );
                    }
                    OutputFormat::Json => {
                        let result = serde_json::json!({
                            "added": true,
                            "type": "file_in_directory",
                            "source": source.display().to_string(),
                            "target_directory": package_dir.display().to_string(),
                            "target_file": target_file.display().to_string()
                        });
                        println!("{}", serde_json::to_string_pretty(&result)?);
                    }
                }
            }
        } else if source.is_dir() {
            // Copy entire directory
            copy_dir_all(source, &package_dir)?;

            let wit_files = count_wit_files(&package_dir)?;

            match format {
                OutputFormat::Human => {
                    eprintln!(
                        "Copied directory: {} -> {}",
                        source.display(),
                        package_dir.display()
                    );
                    eprintln!("   Found {} WIT files", wit_files);
                }
                OutputFormat::Json => {
                    let result = serde_json::json!({
                        "added": true,
                        "type": "directory",
                        "source": source.display().to_string(),
                        "target": package_dir.display().to_string(),
                        "wit_files": wit_files
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
        }
    } else {
        // Create the package directory with placeholder (original behavior)
        std::fs::create_dir_all(&package_dir)?;

        // Create a placeholder WIT file
        let placeholder_content = format!(
            r#"// Placeholder for {}:{} package
// 
// Replace this file with the actual WIT package files.
// You can:
// 1. Download the package from a repository
// 2. Copy WIT files from another location  
// 3. Create the WIT interface definitions manually
// 4. Use --from to copy from local files/directories
//
// Example: wit-bindgen deps --add {}:{} --from /path/to/package
//
// Remember: wit-parser processes dependencies alphabetically!

package {}:{};
"#,
            namespace, name, namespace, name, namespace, name
        );

        let placeholder_file = package_dir.join("placeholder.wit");
        std::fs::write(&placeholder_file, placeholder_content)?;

        match format {
            OutputFormat::Human => {
                eprintln!("Created dependency directory: {}", package_dir.display());
                eprintln!("Added placeholder WIT file.");
                eprintln!("\nNext steps:");
                eprintln!("  1. Replace placeholder.wit with actual WIT package files");
                eprintln!("  2. Or use 'wit-bindgen deps --add {}:{} --from /path/to/package' to copy from local files", namespace, name);
                eprintln!(
                    "  3. Or use 'wit-bindgen deps --fix' to auto-discover existing packages"
                );
                eprintln!("  4. Run 'wit-bindgen deps --check' to verify");
            }
            OutputFormat::Json => {
                let result = serde_json::json!({
                    "added": true,
                    "type": "placeholder",
                    "dependency": {
                        "namespace": namespace,
                        "name": name,
                        "directory": package_dir.display().to_string()
                    },
                    "placeholder_created": placeholder_file.display().to_string(),
                    "next_steps": [
                        "Replace placeholder.wit with actual WIT package files",
                        format!("Or use 'wit-bindgen deps --add {}:{} --from /path/to/package' to copy from local files", namespace, name),
                        "Or use 'wit-bindgen deps --fix' to auto-discover existing packages",
                        "Run 'wit-bindgen deps --check' to verify"
                    ]
                });
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
    }

    // Verify alphabetical ordering
    verify_deps_alphabetical_order(&deps_dir)?;

    Ok(())
}

/// Count WIT files in a directory recursively
fn count_wit_files(dir: &std::path::Path) -> Result<usize> {
    let mut count = 0;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            count += count_wit_files(&path)?;
        } else if path.extension().map(|s| s == "wit").unwrap_or(false) {
            count += 1;
        }
    }

    Ok(count)
}

fn analyze_dependencies(opts: &Common, format: OutputFormat) -> Result<()> {
    let mut resolve = setup_resolve_with_features(opts);
    let deps_dir = opts.wit.parent().unwrap_or(&opts.wit).join("deps");

    let mut analysis = serde_json::json!({
        "deps_directory": {
            "path": deps_dir.display().to_string(),
            "exists": deps_dir.exists(),
        },
        "packages": []
    });

    // Try to parse and analyze dependencies
    match resolve.push_path(&opts.wit) {
        Ok((_pkg_id, _)) => {
            let mut packages = Vec::new();
            for (_id, package) in resolve.packages.iter() {
                packages.push(serde_json::json!({
                    "namespace": package.name.namespace,
                    "name": package.name.name,
                    "full_name": format!("{}:{}", package.name.namespace, package.name.name),
                    "interfaces": package.interfaces.len(),
                    "worlds": package.worlds.len(),
                }));
            }
            analysis["packages"] = serde_json::json!(packages);
            analysis["status"] = serde_json::json!("resolved");
        }
        Err(e) => {
            analysis["status"] = serde_json::json!("error");
            analysis["error"] = serde_json::json!(e.to_string());
        }
    }

    match format {
        OutputFormat::Human => {
            eprintln!("Dependency Analysis");
            eprintln!("==================");
            eprintln!(
                "deps/ directory: {}",
                if deps_dir.exists() {
                    "exists"
                } else {
                    "not found"
                }
            );

            if let Some(packages) = analysis["packages"].as_array() {
                eprintln!("\nFound {} packages:", packages.len());
                for pkg in packages {
                    eprintln!("  - {}", pkg["full_name"].as_str().unwrap_or("unknown"));
                }
            }

            eprintln!("\nUseful commands:");
            eprintln!("  wit-bindgen deps --check     # Check for missing dependencies");
            eprintln!("  wit-bindgen deps --generate  # Create deps/ directory structure");
            eprintln!("  wit-bindgen deps --add <pkg> # Add a dependency");
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&analysis)?);
        }
    }

    Ok(())
}

/// Auto-fix broken dependency paths by scanning for missing packages
fn fix_dependencies(opts: &Common, format: OutputFormat) -> Result<()> {
    let base_dir = opts.wit.parent().unwrap_or(&opts.wit);
    let deps_dir = base_dir.join("deps");

    // Step 1: Try to parse WIT files to identify missing packages
    let mut resolve = setup_resolve_with_features(opts);
    let missing_packages = match resolve.push_path(&opts.wit) {
        Ok(_) => {
            // If parsing succeeds, no missing packages
            Vec::new()
        }
        Err(e) => {
            // Extract package names from error messages
            let error_str = e.to_string();
            let mut packages = Vec::new();

            // Look for common error patterns
            if let Some(pkg) = extract_package_from_error(&error_str) {
                packages.push(pkg);
            }

            packages
        }
    };

    // Step 2: Use DirectoryDependencyScanner for enhanced dependency detection
    let scanner = DirectoryDependencyScanner::new(base_dir);
    let validation_issues = scanner.validate_structure()?;

    // Report validation issues
    for issue in &validation_issues {
        match format {
            OutputFormat::Human => {
                let prefix = match issue.severity {
                    IssueSeverity::Error => "Error:",
                    IssueSeverity::Warning => "Warning:",
                    IssueSeverity::Info => "ℹ️  Info:",
                };
                eprintln!("{} {}", prefix, issue.message);
                eprintln!("   Suggestion: {}", issue.suggestion);
            }
            OutputFormat::Json => {
                // We'll include validation issues in the final JSON output
            }
        }
    }

    // Step 3: Scan for potential package directories (using actual wit-parser resolution search pattern)
    let mut found_fixes = Vec::new();
    let search_dirs = vec![
        base_dir.join("..").join("interfaces"),
        base_dir.join("..").join("packages"),
        base_dir.join("..").join("deps"),
        base_dir.join("../../interfaces"),
        base_dir.join("../../packages"),
    ];

    for missing_pkg in &missing_packages {
        for search_dir in &search_dirs {
            if let Ok(entries) = std::fs::read_dir(search_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let dir_name = path.file_name().unwrap().to_string_lossy();

                        // Check if this directory might contain the missing package
                        if dir_name.contains(&missing_pkg.replace(':', "-"))
                            || dir_name.contains(&missing_pkg.replace(':', "_"))
                            || scan_directory_for_package(&path, missing_pkg)?
                        {
                            found_fixes.push((missing_pkg.clone(), path.clone()));
                            break;
                        }
                    }
                }
            }
        }
    }

    match format {
        OutputFormat::Human => {
            if missing_packages.is_empty() {
                eprintln!("No missing dependencies found!");
                return Ok(());
            }

            eprintln!("Found {} missing packages:", missing_packages.len());
            for pkg in &missing_packages {
                eprintln!("  - {}", pkg);
            }

            if found_fixes.is_empty() {
                eprintln!("\nError: No automatic fixes found.");
                eprintln!("Try:");
                eprintln!("  1. Copy missing packages to deps/ directory");
                eprintln!(
                    "  2. Ensure packages are in alphabetical order (wit-parser requirement)"
                );
                eprintln!("  3. Place as: deps/package-name/ (for directories) or deps/package-name.wit (for files)");
                return Ok(());
            }

            eprintln!("\nFound {} potential fixes:", found_fixes.len());
            for (pkg, path) in &found_fixes {
                eprintln!("  - {} -> {}", pkg, path.display());
            }

            // Apply fixes by creating proper directory structure (NOT deps.toml)
            apply_directory_fixes(&deps_dir, &found_fixes)?;

            eprintln!("\nCreated directory structure in deps/");
            eprintln!("Run 'wit-bindgen validate' to verify the fixes");
        }
        OutputFormat::Json => {
            let result = serde_json::json!({
                "missing_packages": missing_packages,
                "found_fixes": found_fixes.iter().map(|(pkg, path)| {
                    serde_json::json!({
                        "package": pkg,
                        "path": path.display().to_string()
                    })
                }).collect::<Vec<_>>(),
                "validation_issues": validation_issues.iter().map(|issue| {
                    serde_json::json!({
                        "severity": format!("{:?}", issue.severity),
                        "type": format!("{:?}", issue.issue_type),
                        "message": issue.message,
                        "suggestion": issue.suggestion
                    })
                }).collect::<Vec<_>>(),
                "deps_directory_updated": !found_fixes.is_empty()
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}

/// Scan a directory to see if it contains a specific WIT package
fn scan_directory_for_package(dir: &std::path::Path, package_name: &str) -> Result<bool> {
    let wit_files = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "wit")
                .unwrap_or(false)
        });

    for wit_file in wit_files {
        if let Ok(content) = std::fs::read_to_string(wit_file.path()) {
            // Look for package declarations that match our target
            if content.contains(&format!("package {}", package_name)) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// Apply directory fixes by creating proper deps/ directory structure
/// This mirrors how wit-parser's parse_deps_dir() actually works
fn apply_directory_fixes(
    deps_dir: &std::path::Path,
    fixes: &[(String, std::path::PathBuf)],
) -> Result<()> {
    // Ensure deps/ directory exists
    std::fs::create_dir_all(deps_dir)?;

    for (package_name, source_path) in fixes {
        let package_dir_name = package_name.replace(':', "-");
        let target_path = deps_dir.join(&package_dir_name);

        if source_path.is_dir() {
            // Copy entire directory to deps/package-name/
            if !target_path.exists() {
                copy_dir_all(source_path, &target_path)?;
                eprintln!(
                    "  Copied {} -> {}",
                    source_path.display(),
                    target_path.display()
                );
            } else {
                eprintln!(
                    "  Warning: Directory {} already exists",
                    target_path.display()
                );
            }
        } else if source_path.extension().map(|s| s == "wit").unwrap_or(false) {
            // Copy single .wit file to deps/package-name.wit
            let target_file = deps_dir.join(format!("{}.wit", package_dir_name));
            if !target_file.exists() {
                std::fs::copy(source_path, &target_file)?;
                eprintln!(
                    "  📄 Copied {} -> {}",
                    source_path.display(),
                    target_file.display()
                );
            } else {
                eprintln!("  Warning: File {} already exists", target_file.display());
            }
        }
    }

    // Verify alphabetical ordering (critical for wit-parser resolution)
    verify_deps_alphabetical_order(deps_dir)?;

    Ok(())
}

/// Copy a directory recursively
fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;

        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }

    Ok(())
}

/// Verify that deps/ directory contents are in alphabetical order
/// This is critical because wit-parser sorts entries alphabetically
fn verify_deps_alphabetical_order(deps_dir: &std::path::Path) -> Result<()> {
    if !deps_dir.exists() {
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(deps_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();

    let original_order = entries.clone();
    entries.sort();

    if original_order != entries {
        eprintln!("  Warning: deps/ directory not in alphabetical order");
        eprintln!("     Current order: {:?}", original_order);
        eprintln!("     Expected order: {:?}", entries);
        eprintln!("     wit-parser processes dependencies alphabetically!");
    } else {
        eprintln!("  Dependencies are in correct alphabetical order");
    }

    Ok(())
}

/// DirectoryDependencyScanner mirrors wit-parser's parse_deps_dir() functionality
/// This provides intelligent scanning and validation of the deps/ directory structure
pub struct DirectoryDependencyScanner {
    deps_dir: std::path::PathBuf,
}

impl DirectoryDependencyScanner {
    pub fn new(base_dir: &std::path::Path) -> Self {
        Self {
            deps_dir: base_dir.join("deps"),
        }
    }

    /// Scan deps/ directory for packages, mirroring wit-parser's logic
    pub fn scan_packages(&self) -> Result<Vec<DependencyPackage>> {
        let mut packages = Vec::new();

        if !self.deps_dir.exists() {
            return Ok(packages);
        }

        // Mirror wit-parser's parse_deps_dir: read directory and sort alphabetically
        let mut entries: Vec<_> =
            std::fs::read_dir(&self.deps_dir)?.collect::<std::io::Result<Vec<_>>>()?;

        // This is CRITICAL: wit-parser sorts entries alphabetically by filename
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let path = entry.path();
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();

            // Skip hidden files and README
            if filename_str.starts_with('.') || filename_str == "README.md" {
                continue;
            }

            if path.is_dir() {
                // Directory package: deps/package-name/
                if let Some(package) = self.scan_directory_package(&path, &filename_str)? {
                    packages.push(package);
                }
            } else if filename_str.ends_with(".wit") {
                // Single file package: deps/package-name.wit
                if let Some(package) = self.scan_file_package(&path, &filename_str)? {
                    packages.push(package);
                }
            }
            // wit-parser also supports .wasm/.wat files but we'll focus on .wit for now
        }

        Ok(packages)
    }

    fn scan_directory_package(
        &self,
        path: &std::path::Path,
        dir_name: &str,
    ) -> Result<Option<DependencyPackage>> {
        let wit_files: Vec<_> = std::fs::read_dir(path)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == "wit")
                    .unwrap_or(false)
            })
            .collect();

        if wit_files.is_empty() {
            return Ok(None);
        }

        // Try to extract package info from WIT files
        let package_info = self.extract_package_info_from_directory(path)?;

        Ok(Some(DependencyPackage {
            name: package_info.unwrap_or_else(|| dir_name.to_string()),
            path: path.to_path_buf(),
            package_type: DependencyType::Directory,
            wit_files: wit_files.len(),
            alphabetical_position: dir_name.to_string(),
        }))
    }

    fn scan_file_package(
        &self,
        path: &std::path::Path,
        file_name: &str,
    ) -> Result<Option<DependencyPackage>> {
        let package_name = file_name.strip_suffix(".wit").unwrap_or(file_name);
        let package_info = self.extract_package_info_from_file(path)?;

        Ok(Some(DependencyPackage {
            name: package_info.unwrap_or_else(|| package_name.to_string()),
            path: path.to_path_buf(),
            package_type: DependencyType::SingleFile,
            wit_files: 1,
            alphabetical_position: file_name.to_string(),
        }))
    }

    fn extract_package_info_from_directory(
        &self,
        dir_path: &std::path::Path,
    ) -> Result<Option<String>> {
        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            if entry
                .path()
                .extension()
                .map(|s| s == "wit")
                .unwrap_or(false)
            {
                if let Some(package_name) = self.extract_package_info_from_file(&entry.path())? {
                    return Ok(Some(package_name));
                }
            }
        }
        Ok(None)
    }

    fn extract_package_info_from_file(
        &self,
        file_path: &std::path::Path,
    ) -> Result<Option<String>> {
        let content = std::fs::read_to_string(file_path)?;

        // Look for package declaration
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("package ") && line.ends_with(';') {
                let package_part = line
                    .strip_prefix("package ")
                    .unwrap()
                    .strip_suffix(';')
                    .unwrap()
                    .trim();
                return Ok(Some(package_part.to_string()));
            }
        }

        Ok(None)
    }

    /// Validate that the deps/ directory follows wit-parser's requirements
    pub fn validate_structure(&self) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();

        if !self.deps_dir.exists() {
            return Ok(issues);
        }

        let packages = self.scan_packages()?;

        // Check alphabetical ordering
        let mut file_names: Vec<_> = packages
            .iter()
            .map(|p| p.alphabetical_position.clone())
            .collect();
        let original_order = file_names.clone();
        file_names.sort();

        if original_order != file_names {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Warning,
                issue_type: IssueType::AlphabeticalOrdering,
                message: format!(
                    "Dependencies not in alphabetical order. Expected: {:?}, Found: {:?}",
                    file_names, original_order
                ),
                suggestion: "Rename files/directories to maintain alphabetical order".to_string(),
            });
        }

        // Check for empty packages
        for package in &packages {
            if package.wit_files == 0 {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Error,
                    issue_type: IssueType::EmptyPackage,
                    message: format!("Package '{}' contains no WIT files", package.name),
                    suggestion: "Add WIT files or remove the empty directory".to_string(),
                });
            }
        }

        Ok(issues)
    }

    /// Test if scanned packages can actually be parsed and resolved by wit-parser
    /// This bridges the gap between directory scanning and actual resolution
    fn validate_packages_with_parser(&self, opts: &Common) -> Result<PackageValidationReport> {
        let mut report = PackageValidationReport {
            total_packages: 0,
            parseable_packages: 0,
            unparseable_packages: Vec::new(),
            parsing_errors: Vec::new(),
            resolution_capable: false,
            resolution_error: None,
        };

        // First scan packages using our directory scanner
        let scanned_packages = self.scan_packages()?;
        report.total_packages = scanned_packages.len();

        // Test individual package parsing
        for package in &scanned_packages {
            match self.test_individual_package_parsing(&package) {
                Ok(_) => {
                    report.parseable_packages += 1;
                }
                Err(e) => {
                    report.unparseable_packages.push(package.name.clone());
                    report
                        .parsing_errors
                        .push(format!("Package '{}': {}", package.name, e));
                }
            }
        }

        // Test full resolution with wit-parser
        match self.test_full_resolution_with_wit_file(opts) {
            Ok(_) => {
                report.resolution_capable = true;
            }
            Err(e) => {
                report.resolution_error = Some(format!("Full resolution failed: {}", e));
            }
        }

        Ok(report)
    }

    /// Test if an individual package can be parsed by wit-parser
    fn test_individual_package_parsing(&self, package: &DependencyPackage) -> Result<()> {
        match package.package_type {
            DependencyType::Directory => {
                // For directory packages, try to parse as UnresolvedPackageGroup
                let mut resolve = wit_parser::Resolve::new();
                match resolve.push_dir(&package.path) {
                    Ok(_) => Ok(()),
                    Err(e) => bail!("Directory parsing failed: {}", e),
                }
            }
            DependencyType::SingleFile => {
                // For single file packages, parse individual file
                let mut resolve = wit_parser::Resolve::new();
                match resolve.push_file(&package.path) {
                    Ok(_) => Ok(()),
                    Err(e) => bail!("File parsing failed: {}", e),
                }
            }
        }
    }

    /// Test if the full deps/ directory can be resolved together with the main WIT file
    fn test_full_resolution_with_wit_file(&self, opts: &Common) -> Result<()> {
        let mut resolve = setup_resolve_with_features(opts);

        // Try to resolve the main WIT file with all dependencies
        match resolve.push_path(&opts.wit) {
            Ok(_) => Ok(()),
            Err(e) => {
                // Extract more detailed error information
                let error_str = format!("{}", e);
                if error_str.contains("not found") {
                    bail!("Dependency resolution failed: Some required packages are missing or cannot be found in deps/");
                } else if error_str.contains("parse") {
                    bail!("Parsing failed: WIT syntax or semantic errors in dependencies");
                } else {
                    bail!("Resolution failed: {}", e);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PackageValidationReport {
    pub total_packages: usize,
    pub parseable_packages: usize,
    pub unparseable_packages: Vec<String>,
    pub parsing_errors: Vec<String>,
    pub resolution_capable: bool,
    pub resolution_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DependencyPackage {
    pub name: String,
    pub path: std::path::PathBuf,
    pub package_type: DependencyType,
    pub wit_files: usize,
    pub alphabetical_position: String,
}

#[derive(Debug, Clone)]
pub enum DependencyType {
    Directory,
    SingleFile,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub issue_type: IssueType,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Clone)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub enum IssueType {
    AlphabeticalOrdering,
    EmptyPackage,
    MissingDependency,
    InvalidStructure,
}

/// Check if deps/ directory structure is synchronized with WIT imports
fn check_dependency_sync(opts: &Common, format: OutputFormat) -> Result<()> {
    let base_dir = opts.wit.parent().unwrap_or(&opts.wit);
    let scanner = DirectoryDependencyScanner::new(base_dir);

    // Scan actual packages in deps/
    let found_packages = scanner.scan_packages()?;
    let validation_issues = scanner.validate_structure()?;

    // NEW: Test if scanned packages can actually be parsed by wit-parser
    let parser_validation = scanner.validate_packages_with_parser(opts)?;

    // Try to parse WIT files to find import statements
    let mut resolve = setup_resolve_with_features(opts);
    // This variable is shadowed below in the match statement

    let (expected_imports, parse_status) = match resolve.push_path(&opts.wit) {
        Ok((pkg, _)) => {
            // Extract import information from the resolved package
            let mut imports = Vec::new();

            // Get imports from the main world
            let main_world_id = resolve.select_world(pkg, None)?;
            let world = &resolve.worlds[main_world_id];
            for (_, import) in world.imports.iter() {
                match import {
                    wit_parser::WorldItem::Interface { id, .. } => {
                        if let Some(pkg_id) = resolve.interfaces[*id].package {
                            let package = &resolve.packages[pkg_id];
                            let import_name =
                                format!("{}:{}", package.name.namespace, package.name.name);
                            if !imports.contains(&import_name) {
                                imports.push(import_name);
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Also check for use statements in interfaces
            for (_, interface) in resolve.interfaces.iter() {
                if let Some(pkg_id) = interface.package {
                    let package = &resolve.packages[pkg_id];
                    let import_name = format!("{}:{}", package.name.namespace, package.name.name);
                    if !imports.contains(&import_name) {
                        imports.push(import_name);
                    }
                }
            }

            (imports, "parsed_successfully".to_string())
        }
        Err(parse_error) => {
            // If we can't parse, try manual extraction from WIT files
            let manual_imports = extract_imports_from_wit_files(&opts.wit)?;
            let error_msg = format!("wit-parser failed (using fallback): {}", parse_error);
            (manual_imports, error_msg)
        }
    };

    // Compare found vs expected with improved matching
    let found_names: Vec<String> = found_packages.iter().map(|p| p.name.clone()).collect();

    let missing_in_deps: Vec<_> = expected_imports
        .iter()
        .filter(|import| !is_package_available(import, &found_names))
        .collect();

    let extra_in_deps: Vec<_> = found_names
        .iter()
        .filter(|found| {
            !expected_imports
                .iter()
                .any(|import| is_package_match(import, found))
        })
        .collect();

    match format {
        OutputFormat::Human => {
            eprintln!("Dependency Synchronization Check");
            eprintln!("================================");
            eprintln!("Parse Status: {}", parse_status);
            eprintln!("");
            eprintln!("Expected imports: {} found", expected_imports.len());
            for import in &expected_imports {
                eprintln!("  - {}", import);
            }
            eprintln!("");
            eprintln!("Available packages: {} found", found_names.len());
            for package in &found_names {
                eprintln!("  - {}", package);
            }
            eprintln!("");

            if !missing_in_deps.is_empty() {
                eprintln!("\nError: Missing in deps/ directory:");
                for missing in &missing_in_deps {
                    eprintln!("  ✗ {}", missing);
                }
            }

            if !extra_in_deps.is_empty() {
                eprintln!("\nWarning: Extra packages in deps/ (not imported):");
                for extra in &extra_in_deps {
                    eprintln!("  ? {}", extra);
                }
            }

            // NEW: Display parser validation results
            eprintln!("\nParser Integration Validation:");
            eprintln!("  Packages found: {}", parser_validation.total_packages);
            eprintln!(
                "  Parseable by wit-parser: {}",
                parser_validation.parseable_packages
            );

            if !parser_validation.unparseable_packages.is_empty() {
                eprintln!("  Error: Unparseable packages:");
                for pkg in &parser_validation.unparseable_packages {
                    eprintln!("    ✗ {}", pkg);
                }
            }

            if !parser_validation.parsing_errors.is_empty() {
                eprintln!("  🐛 Parsing errors:");
                for error in &parser_validation.parsing_errors {
                    eprintln!("    - {}", error);
                }
            }

            eprintln!(
                "  Full resolution: {}",
                if parser_validation.resolution_capable {
                    "Success"
                } else {
                    "Failed"
                }
            );

            if let Some(ref resolution_error) = parser_validation.resolution_error {
                eprintln!("  Resolution error: {}", resolution_error);
            }

            if !validation_issues.is_empty() {
                eprintln!("\nStructure issues:");
                for issue in &validation_issues {
                    let prefix = match issue.severity {
                        IssueSeverity::Error => "Error:",
                        IssueSeverity::Warning => "Warning:",
                        IssueSeverity::Info => "ℹ️  Info:",
                    };
                    eprintln!("  {} {}", prefix, issue.message);
                    eprintln!("     Suggestion: {}", issue.suggestion);
                }
            }

            if missing_in_deps.is_empty()
                && extra_in_deps.is_empty()
                && validation_issues.is_empty()
                && parser_validation.resolution_capable
            {
                eprintln!("\nDependencies are properly synchronized and fully resolvable!");
            } else if !parser_validation.resolution_capable {
                eprintln!(
                    "\nError: Dependencies have resolution issues that prevent proper operation!"
                );
            }
        }
        OutputFormat::Json => {
            let result = serde_json::json!({
                "synchronized": missing_in_deps.is_empty() && extra_in_deps.is_empty(),
                "fully_resolvable": parser_validation.resolution_capable,
                "parse_status": parse_status,
                "expected_imports": expected_imports,
                "found_packages": found_packages.iter().map(|p| {
                    serde_json::json!({
                        "name": p.name,
                        "wit_files": p.wit_files,
                        "type": format!("{:?}", p.package_type)
                    })
                }).collect::<Vec<_>>(),
                "extra_packages": extra_in_deps,
                "missing_in_deps": missing_in_deps,
                "parser_validation": {
                    "total_packages": parser_validation.total_packages,
                    "parseable_packages": parser_validation.parseable_packages,
                    "unparseable_packages": parser_validation.unparseable_packages,
                    "parsing_errors": parser_validation.parsing_errors,
                    "resolution_capable": parser_validation.resolution_capable,
                    "resolution_error": parser_validation.resolution_error
                },
                "extra_in_deps": extra_in_deps,
                "validation_issues": validation_issues.iter().map(|issue| {
                    serde_json::json!({
                        "severity": format!("{:?}", issue.severity),
                        "type": format!("{:?}", issue.issue_type),
                        "message": issue.message,
                        "suggestion": issue.suggestion
                    })
                }).collect::<Vec<_>>()
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}

/// Fix alphabetical ordering issues in deps/ directory
fn fix_alphabetical_ordering(opts: &Common, format: OutputFormat) -> Result<()> {
    let base_dir = opts.wit.parent().unwrap_or(&opts.wit);
    let deps_dir = base_dir.join("deps");
    let scanner = DirectoryDependencyScanner::new(base_dir);

    if !deps_dir.exists() {
        match format {
            OutputFormat::Human => {
                eprintln!("No deps/ directory found. Nothing to fix.");
            }
            OutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::json!({
                        "fixed": false,
                        "reason": "No deps/ directory found"
                    })
                );
            }
        }
        return Ok(());
    }

    let packages = scanner.scan_packages()?;
    let mut file_names: Vec<_> = packages
        .iter()
        .map(|p| p.alphabetical_position.clone())
        .collect();
    let original_order = file_names.clone();
    file_names.sort();

    if original_order == file_names {
        match format {
            OutputFormat::Human => {
                eprintln!("Dependencies are already in correct alphabetical order!");
            }
            OutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::json!({
                        "fixed": false,
                        "reason": "Already in correct order",
                        "current_order": original_order
                    })
                );
            }
        }
        return Ok(());
    }

    match format {
        OutputFormat::Human => {
            eprintln!("Fixing alphabetical ordering...");
            eprintln!("Current order: {:?}", original_order);
            eprintln!("Correct order: {:?}", file_names);
            eprintln!("\nWarning: Automatic reordering of existing files/directories is complex.");
            eprintln!("Manual action required:");
            eprintln!("  1. Backup your deps/ directory");
            eprintln!("  2. Rename files/directories to match alphabetical order:");

            for (i, correct_name) in file_names.iter().enumerate() {
                if i < original_order.len() && &original_order[i] != correct_name {
                    eprintln!(
                        "     - Rename '{}' -> position {}",
                        original_order[i],
                        i + 1
                    );
                }
            }

            eprintln!("  3. Or recreate dependencies using 'wit-bindgen deps --add' commands");
        }
        OutputFormat::Json => {
            let fixes: Vec<_> = original_order
                .iter()
                .enumerate()
                .filter_map(|(i, current)| {
                    if i < file_names.len() && current != &file_names[i] {
                        Some(serde_json::json!({
                            "current_name": current,
                            "suggested_position": i + 1,
                            "target_position_name": file_names.get(i)
                        }))
                    } else {
                        None
                    }
                })
                .collect();

            println!(
                "{}",
                serde_json::json!({
                    "fixed": false,
                    "reason": "Manual intervention required",
                    "current_order": original_order,
                    "correct_order": file_names,
                    "suggested_fixes": fixes
                })
            );
        }
    }

    Ok(())
}

/// Extract import statements from WIT files manually (fallback when parsing fails)
fn extract_imports_from_wit_files(wit_path: &std::path::Path) -> Result<Vec<String>> {
    let mut imports = Vec::new();

    if wit_path.is_file() {
        imports.extend(extract_imports_from_single_file(wit_path)?);
    } else if wit_path.is_dir() {
        for entry in std::fs::read_dir(wit_path)? {
            let entry = entry?;
            if entry
                .path()
                .extension()
                .map(|s| s == "wit")
                .unwrap_or(false)
            {
                imports.extend(extract_imports_from_single_file(&entry.path())?);
            }
        }
    }

    imports.sort();
    imports.dedup();
    Ok(imports)
}

fn extract_imports_from_single_file(file_path: &std::path::Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(file_path)?;
    let mut imports = Vec::new();

    // Parse both "use" statements and "import" statements
    let mut in_world = false;
    let mut brace_depth = 0;

    for line in content.lines() {
        let line = line.trim();

        // Track world context
        if line.starts_with("world ") {
            in_world = true;
            brace_depth = 0;
        }

        // Track brace depth to know when we're inside world
        for ch in line.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => {
                    brace_depth -= 1;
                    if brace_depth == 0 && in_world {
                        in_world = false;
                    }
                }
                _ => {}
            }
        }

        // Extract use statements (interface-level)
        if line.starts_with("use ") && line.contains(':') {
            if let Some(import_part) = extract_package_from_line(line, "use ") {
                imports.push(import_part);
            }
        }

        // Extract import statements (world-level)
        if in_world && line.starts_with("import ") && line.contains(':') {
            if let Some(import_part) = extract_package_from_line(line, "import ") {
                imports.push(import_part);
            }
        }
    }

    // Remove duplicates and sort
    imports.sort();
    imports.dedup();

    Ok(imports)
}

fn is_package_available(import: &str, found_packages: &[String]) -> bool {
    found_packages
        .iter()
        .any(|found| is_package_match(import, found))
}

fn is_package_match(import: &str, found: &str) -> bool {
    // Direct match
    if import == found {
        return true;
    }

    // Convert namespace:name to directory names (namespace-name or namespace_name)
    let import_variants = [import.replace(':', "-"), import.replace(':', "_")];

    for variant in &import_variants {
        if found == variant || found.starts_with(&format!("{}/", variant)) {
            return true;
        }
    }

    // Check if found package contains the base name
    if let Some(base_name) = import.split(':').last() {
        if found.contains(base_name) {
            return true;
        }
    }

    false
}

fn extract_package_from_line(line: &str, prefix: &str) -> Option<String> {
    let line = line.strip_prefix(prefix)?.trim();

    // Handle different patterns:
    // "import wasi:http/types@0.2.0;"
    // "use namespace:name;"
    // "import namespace:name as alias;"

    let import_part = if let Some(semicolon_pos) = line.find(';') {
        &line[..semicolon_pos]
    } else {
        line
    }
    .trim();

    // Extract just the package part (before any '@' version specifier or 'as' alias)
    let package_part = import_part
        .split_whitespace()
        .next()?
        .split('@')
        .next()?
        .split('/')
        .next()?;

    if package_part.contains(':') && !package_part.starts_with('@') {
        Some(package_part.to_string())
    } else {
        None
    }
}

/// Enhanced error categorization for dependency resolution failures
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum DependencyResolutionError {
    PackageNotFound {
        package: String,
        context: String,
        searched_locations: Vec<String>,
    },
    InvalidPackageStructure {
        path: String,
        reason: String,
        suggestions: Vec<String>,
    },
    ParseError {
        file: String,
        message: String,
        line_number: Option<u32>,
    },
    WorldNotFound {
        world: String,
        available_worlds: Vec<String>,
    },
    CircularDependency {
        chain: Vec<String>,
    },
    VersionConflict {
        package: String,
        required: String,
        found: String,
    },
    GenericError {
        message: String,
        category: String,
    },
}

/// Comprehensive error analysis and suggestion generation
fn analyze_dependency_error(error_str: &str, context: &Common) -> DependencyResolutionError {
    let base_dir = context.wit.parent().unwrap_or(&context.wit);
    let _deps_dir = base_dir.join("deps");

    // Enhanced package name extraction with context
    if let Some(package) = extract_package_from_error_enhanced(error_str) {
        let searched_locations = vec![
            format!("deps/{}.wit", package.replace(':', "-")),
            format!("deps/{}.wit", package.replace(':', "_")),
            format!("deps/{}/", package.replace(':', "-")),
            format!("deps/{}/", package.replace(':', "_")),
        ];

        return DependencyResolutionError::PackageNotFound {
            package,
            context: format!("Referenced in {}", context.wit.display()),
            searched_locations,
        };
    }

    // Parse error detection with line numbers
    if error_str.contains("-->") && error_str.contains(":") {
        if let Some(parse_info) = extract_parse_error_info(error_str) {
            return DependencyResolutionError::ParseError {
                file: parse_info.0,
                message: parse_info.1,
                line_number: parse_info.2,
            };
        }
    }

    // World not found detection
    if error_str.contains("world") && error_str.contains("not found") {
        return DependencyResolutionError::WorldNotFound {
            world: extract_world_from_error(error_str).unwrap_or("unknown".to_string()),
            available_worlds: vec![], // Could be enhanced to list available worlds
        };
    }

    // Directory resolution errors
    if error_str.contains("failed to resolve directory") {
        let suggestions = vec![
            "Check that the WIT directory exists and contains .wit files".to_string(),
            "Verify file permissions on the WIT directory".to_string(),
            "Ensure the path is correct and accessible".to_string(),
        ];

        return DependencyResolutionError::InvalidPackageStructure {
            path: context.wit.display().to_string(),
            reason: "Directory resolution failed".to_string(),
            suggestions,
        };
    }

    // Generic categorization
    let category = if error_str.contains("syntax") {
        "syntax_error"
    } else if error_str.contains("interface") {
        "interface_error"
    } else if error_str.contains("type") {
        "type_error"
    } else {
        "unknown_error"
    };

    DependencyResolutionError::GenericError {
        message: error_str.to_string(),
        category: category.to_string(),
    }
}

/// Generate actionable suggestions based on the error type and current project state
fn generate_actionable_suggestions(
    error: &DependencyResolutionError,
    opts: &Common,
) -> Vec<String> {
    match error {
        DependencyResolutionError::PackageNotFound {
            package,
            searched_locations,
            ..
        } => {
            let mut suggestions = vec![
                format!(
                    "Add the missing package '{}' to the deps/ directory",
                    package
                ),
                "Check the package name spelling and namespace".to_string(),
            ];

            // Suggest specific file locations
            suggestions.push(format!("Create one of these files:"));
            for location in searched_locations {
                suggestions.push(format!("  - {}", location));
            }

            // Suggest using wit-bindgen deps command
            if package.contains(':') {
                suggestions.push(format!(
                    "Use: wit-bindgen deps --add {} --from <source> {}",
                    package,
                    opts.wit.display()
                ));
            }

            suggestions
        }

        DependencyResolutionError::ParseError {
            file, line_number, ..
        } => {
            let mut suggestions = vec![
                format!("Fix syntax error in {}", file),
                "Check WIT syntax documentation".to_string(),
            ];

            if let Some(line) = line_number {
                suggestions.push(format!("Focus on line {} in {}", line, file));
            }

            suggestions
                .push("Run: wit-bindgen validate --analyze for detailed diagnostics".to_string());
            suggestions
        }

        DependencyResolutionError::WorldNotFound { world, .. } => {
            vec![
                format!("Check that world '{}' exists in the WIT file", world),
                "List available worlds with: wit-bindgen validate --show-tree".to_string(),
                "Remove the --world flag to use the default world".to_string(),
            ]
        }

        DependencyResolutionError::InvalidPackageStructure { suggestions, .. } => {
            let mut result = suggestions.clone();
            result.push("Run: wit-bindgen deps --sync-check for structure validation".to_string());
            result
        }

        DependencyResolutionError::CircularDependency { chain } => {
            vec![
                "Break the circular dependency by restructuring packages".to_string(),
                format!("Dependency chain: {}", chain.join(" -> ")),
                "Consider extracting common interfaces to a separate package".to_string(),
            ]
        }

        DependencyResolutionError::VersionConflict {
            package,
            required,
            found,
        } => {
            vec![
                format!(
                    "Update package '{}' from version {} to {}",
                    package, found, required
                ),
                "Check version compatibility in your deps/ directory".to_string(),
                "Consider using version ranges instead of exact versions".to_string(),
            ]
        }

        DependencyResolutionError::GenericError { category, .. } => match category.as_str() {
            "syntax_error" => vec![
                "Check WIT file syntax".to_string(),
                "Verify all braces, semicolons, and keywords are correct".to_string(),
                "Run: wit-bindgen validate for detailed syntax checking".to_string(),
            ],
            "interface_error" => vec![
                "Check that all referenced interfaces are defined".to_string(),
                "Verify interface names match exactly".to_string(),
                "Run: wit-bindgen deps --sync-check to verify dependencies".to_string(),
            ],
            "type_error" => vec![
                "Check that all referenced types are defined".to_string(),
                "Verify type names and imports".to_string(),
                "Ensure all dependencies are available in deps/".to_string(),
            ],
            _ => vec![
                "Run: wit-bindgen validate --analyze for detailed error analysis".to_string(),
                "Check wit-bindgen help-ai for comprehensive documentation".to_string(),
            ],
        },
    }
}

fn extract_parse_error_info(error_str: &str) -> Option<(String, String, Option<u32>)> {
    // Extract file, message, and line number from wit-parser error format
    // Example: " --> example.wit:4:10"
    if let Some(arrow_pos) = error_str.find("-->") {
        let location_part = &error_str[arrow_pos + 3..].trim();
        if let Some(colon_pos) = location_part.find(':') {
            let file = location_part[..colon_pos].trim().to_string();
            let rest = &location_part[colon_pos + 1..];
            if let Some(second_colon) = rest.find(':') {
                if let Ok(line_num) = rest[..second_colon].parse::<u32>() {
                    let message = error_str
                        .lines()
                        .next()
                        .unwrap_or("Parse error")
                        .to_string();
                    return Some((file, message, Some(line_num)));
                }
            }
        }
    }
    None
}

fn extract_world_from_error(error_str: &str) -> Option<String> {
    // Extract world name from error messages like "world 'my-world' not found"
    if let Some(start) = error_str.find("world '") {
        if let Some(end) = error_str[start + 7..].find("'") {
            return Some(error_str[start + 7..start + 7 + end].to_string());
        }
    }
    None
}

fn extract_package_from_error_enhanced(error_str: &str) -> Option<String> {
    // Enhanced package extraction supporting more error patterns without regex

    // Pattern 1: package 'name'
    if let Some(start) = error_str.find("package '") {
        if let Some(end) = error_str[start + 9..].find("'") {
            let package = &error_str[start + 9..start + 9 + end];
            return Some(package.split('@').next().unwrap_or(package).to_string());
        }
    }

    // Pattern 2: package `name`
    if let Some(start) = error_str.find("package `") {
        if let Some(end) = error_str[start + 9..].find("`") {
            let package = &error_str[start + 9..start + 9 + end];
            return Some(package.split('@').next().unwrap_or(package).to_string());
        }
    }

    // Pattern 3: Look for namespace:name patterns
    for word in error_str.split_whitespace() {
        if word.contains(':') && !word.starts_with('@') {
            // Remove quotes, trailing punctuation, and version specifiers
            let clean_word = word
                .trim_matches(|c: char| !c.is_alphanumeric() && c != ':' && c != '-' && c != '_');
            let package = clean_word.split('@').next().unwrap_or(clean_word);
            if package.matches(':').count() == 1 && package.len() > 3 {
                return Some(package.to_string());
            }
        }
    }

    // Fallback to original extraction method
    extract_package_from_error(error_str)
}

fn extract_package_from_error(error_str: &str) -> Option<String> {
    // Try to extract package name from common error patterns
    if let Some(start) = error_str.find("package '") {
        if let Some(end) = error_str[start + 9..].find("'") {
            return Some(error_str[start + 9..start + 9 + end].to_string());
        }
    }

    if let Some(start) = error_str.find("package `") {
        if let Some(end) = error_str[start + 9..].find("`") {
            return Some(error_str[start + 9..start + 9 + end].to_string());
        }
    }

    None
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Opt::command().debug_assert()
}
