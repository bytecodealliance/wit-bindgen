use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use rayon::prelude::*;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io::Write;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use wasm_encoder::{Encode, Section};
use wit_component::{ComponentEncoder, StringEncoding};

mod c;
mod config;
mod cpp;
mod csharp;
mod custom;
mod moonbit;
mod runner;
mod rust;
mod wat;

/// Tool to run tests that exercise the `wit-bindgen` bindings generator.
///
/// This tool is used to (a) generate bindings for a target language, (b)
/// compile the bindings and source code to a wasm component, (c) compose a
/// "runner" and a "test" component together, and (d) execute this component to
/// ensure that it passes. This process is guided by filesystem structure which
/// must adhere to some conventions.
///
/// * Tests are located in any directory that contains a `test.wit` description
///   of the WIT being tested. The `<TEST>` argument to this command is walked
///   recursively to find `test.wit` files.
///
/// * The `test.wit` file must have a `runner` world and a `test` world. The
///   "runner" should import interfaces that are exported by "test".
///
/// * Adjacent to `test.wit` should be a number of `runner*.*` files. There is
///   one runner per source language, for example `runner.rs` and `runner.c`.
///   These are source files for the `runner` world. Source files can start with
///   `//@ ...` comments to deserialize into `config::RuntimeTestConfig`,
///   currently that supports:
///
///   ```text
///   //@ args = ['--arguments', 'to', '--the', 'bindings', '--generator']
///   ```
///
///   or
///
///   ```text
///   //@ args = '--arguments to --the bindings --generator'
///   ```
///
/// * Adjacent to `test.wit` should also be a number of `test*.*` files. Like
///   runners there is one per source language. Note that you can have multiple
///   implementations of tests in the same language too, for example
///   `test-foo.rs` and `test-bar.rs`. All tests must export the same `test`
///   world from `test.wit`, however.
///
/// This tool will discover `test.wit` files, discover runners/tests, and then
/// compile everything and run the combinatorial matrix of runners against
/// tests. It's expected that each `runner.*` and `test.*` perform the same
/// functionality and only differ in source language.
#[derive(Default, Debug, Clone, Parser)]
pub struct Opts {
    /// Directory containing the test being run or all tests being run.
    test: Vec<PathBuf>,

    /// Path to where binary artifacts for tests are stored.
    #[clap(long, value_name = "PATH")]
    artifacts: PathBuf,

    /// Optional filter to use on test names to only run some tests.
    ///
    /// This is a regular expression defined by the `regex` Rust crate.
    #[clap(short, long, value_name = "REGEX")]
    filter: Option<regex::Regex>,

    /// The executable or script used to execute a fully composed test case.
    #[clap(long, default_value = "wasmtime")]
    runner: std::ffi::OsString,

    #[clap(flatten)]
    rust: rust::RustOpts,

    #[clap(flatten)]
    c: c::COpts,

    #[clap(flatten)]
    custom: custom::CustomOpts,

    /// Whether or not the calling process's stderr is inherited into child
    /// processes.
    ///
    /// This helps preserving color in compiler error messages but can also
    /// jumble up output if there are multiple errors.
    #[clap(short, long)]
    inherit_stderr: bool,

    /// Configuration of which languages are tested.
    ///
    /// Passing `--lang rust` will only test Rust for example.
    #[clap(short, long, required = true, value_delimiter = ',')]
    languages: Vec<String>,

    /// Generate code for symmetric ABI and compile to native
    #[clap(short, long)]
    symmetric: bool,
}

impl Opts {
    pub fn run(&self, wit_bindgen: &Path) -> Result<()> {
        Runner {
            opts: self,
            rust_state: None,
            cpp_state: None,
            wit_bindgen,
            test_runner: runner::TestRunner::new(&self.runner)?,
        }
        .run()
    }
}

/// Helper structure representing a discovered `test.wit` file.
struct Test {
    /// The name of this test, unique amongst all tests.
    ///
    /// Inferred from the directory name.
    name: String,

    /// Path to the root of this test.
    path: PathBuf,

    /// Configuration for this test, specified in the WIT file.
    config: config::WitConfig,

    kind: TestKind,
}

enum TestKind {
    Runtime(Vec<Component>),
    Codegen(PathBuf),
}

/// Helper structure representing a single component found in a test directory.
struct Component {
    /// The name of this component, inferred from the file stem.
    ///
    /// May be shared across different languages.
    name: String,

    /// The path to the source file for this component.
    path: PathBuf,

    /// Whether or not this component is a "runner" or a "test"
    kind: Kind,

    /// The detected language for this component.
    language: Language,

    /// The WIT world that's being used with this component, loaded from
    /// `test.wit`.
    bindgen: Bindgen,

    /// The contents of the test file itself.
    contents: String,

    /// The contents of the test file itself.
    lang_config: Option<HashMap<String, toml::Value>>,
}

#[derive(Clone)]
struct Bindgen {
    /// The arguments to the bindings generator that this component will be
    /// using.
    args: Vec<String>,
    /// The path to the `*.wit` file or files that are having bindings
    /// generated.
    wit_path: PathBuf,
    /// The name of the world within `wit_path` that's having bindings generated
    /// for it.
    world: String,
    /// Configuration found in `wit_path`
    wit_config: config::WitConfig,
}

#[derive(Debug, PartialEq, Copy, Clone)]
enum Kind {
    Runner,
    Test,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Language {
    Rust,
    C,
    Cpp,
    Cpp17,
    Wat,
    Csharp,
    MoonBit,
    Custom(custom::Language),
}

/// Helper structure to package up arguments when sent to language-specific
/// compilation backends for `LanguageMethods::compile`
struct Compile<'a> {
    component: &'a Component,
    bindings_dir: &'a Path,
    artifacts_dir: &'a Path,
    output: &'a Path,
}

/// Helper structure to package up arguments when sent to language-specific
/// compilation backends for `LanguageMethods::verify`
struct Verify<'a> {
    wit_test: &'a Path,
    bindings_dir: &'a Path,
    artifacts_dir: &'a Path,
    args: &'a [String],
    world: &'a str,
}

/// Helper structure to package up runtime state associated with executing tests.
struct Runner<'a> {
    opts: &'a Opts,
    rust_state: Option<rust::State>,
    cpp_state: Option<cpp::State>,
    wit_bindgen: &'a Path,
    test_runner: runner::TestRunner,
}

impl Runner<'_> {
    /// Executes all tests.
    fn run(&mut self) -> Result<()> {
        // First step, discover all tests in the specified test directory.
        let mut tests = HashMap::new();
        for test in self.opts.test.iter() {
            self.discover_tests(&mut tests, test)
                .with_context(|| format!("failed to discover tests in {test:?}"))?;
        }
        if tests.is_empty() {
            bail!(
                "no `test.wit` files found were found in {:?}",
                self.opts.test,
            );
        }

        self.prepare_languages(&tests)?;
        self.run_codegen_tests(&tests)?;
        self.run_runtime_tests(&tests)?;

        println!("PASSED");

        Ok(())
    }

    /// Walks over `dir`, recursively, inserting located cases into `tests`.
    fn discover_tests(&self, tests: &mut HashMap<String, Test>, path: &Path) -> Result<()> {
        if path.is_file() {
            if path.extension().and_then(|s| s.to_str()) == Some("wit") {
                let config =
                    fs::read_to_string(path).with_context(|| format!("failed to read {path:?}"))?;
                let config = config::parse_test_config::<config::WitConfig>(&config, "//@")
                    .with_context(|| format!("failed to parse test config from {path:?}"))?;
                return self.insert_test(&path, config, TestKind::Codegen(path.to_owned()), tests);
            }

            return Ok(());
        }

        let runtime_candidate = path.join("test.wit");
        if runtime_candidate.is_file() {
            let (config, components) = self
                .load_runtime_test(&runtime_candidate, path)
                .with_context(|| format!("failed to load test in {path:?}"))?;
            return self.insert_test(path, config, TestKind::Runtime(components), tests);
        }

        let codegen_candidate = path.join("wit");
        if codegen_candidate.is_dir() {
            return self.insert_test(
                path,
                Default::default(),
                TestKind::Codegen(codegen_candidate),
                tests,
            );
        }

        for entry in path.read_dir().context("failed to read test directory")? {
            let entry = entry.context("failed to read test directory entry")?;
            let path = entry.path();

            self.discover_tests(tests, &path)?;
        }

        Ok(())
    }

    fn insert_test(
        &self,
        path: &Path,
        config: config::WitConfig,
        kind: TestKind,
        tests: &mut HashMap<String, Test>,
    ) -> Result<()> {
        let test_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .context("non-utf-8 filename")?;
        let prev = tests.insert(
            test_name.to_string(),
            Test {
                name: test_name.to_string(),
                path: path.to_path_buf(),
                config,
                kind,
            },
        );
        if prev.is_some() {
            bail!("duplicate test name `{test_name}` found");
        }
        Ok(())
    }

    /// Loads a test from `dir` using the `wit` file in the directory specified.
    ///
    /// Returns a list of components that were found within this directory.
    fn load_runtime_test(
        &self,
        wit: &Path,
        dir: &Path,
    ) -> Result<(config::WitConfig, Vec<Component>)> {
        let mut resolve = wit_parser::Resolve::default();

        let wit_path = if dir.join("deps").exists() { dir } else { wit };
        let (pkg, _files) = resolve.push_path(wit_path).context(format!(
            "failed to load `test.wit` in test directory: {:?}",
            &wit
        ))?;
        let resolve = Arc::new(resolve);

        let wit_contents = std::fs::read_to_string(wit)?;
        let wit_config: config::WitConfig = config::parse_test_config(&wit_contents, "//@")
            .context("failed to parse WIT test config")?;

        let mut worlds = Vec::new();

        let mut push_world = |kind: Kind, name: &str| -> Result<()> {
            let world = resolve.select_world(pkg, Some(name)).with_context(|| {
                format!("failed to find expected `{name}` world to generate bindings")
            })?;
            worlds.push((world, kind));
            Ok(())
        };
        push_world(Kind::Runner, wit_config.runner_world())?;
        for world in wit_config.dependency_worlds() {
            push_world(Kind::Test, &world)?;
        }

        let mut components = Vec::new();
        let mut any_runner = false;
        let mut any_test = false;

        for entry in dir.read_dir().context("failed to read test directory")? {
            let entry = entry.context("failed to read test directory entry")?;
            let path = entry.path();

            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if name == "test.wit" {
                continue;
            }

            let Some((world, kind)) = worlds
                .iter()
                .find(|(world, _kind)| name.starts_with(&resolve.worlds[*world].name))
            else {
                log::debug!("skipping file {name:?}");
                continue;
            };
            match kind {
                Kind::Runner => any_runner = true,
                Kind::Test => any_test = true,
            }
            let bindgen = Bindgen {
                args: Vec::new(),
                wit_config: wit_config.clone(),
                world: resolve.worlds[*world].name.clone(),
                wit_path: wit_path.to_path_buf(),
            };
            let component = self
                .parse_component(&path, *kind, bindgen)
                .with_context(|| format!("failed to parse component source file {path:?}"))?;
            components.push(component);
        }

        if !any_runner {
            bail!("no runner files found in test directory");
        }
        if !any_test {
            bail!("no test files found in test directory");
        }

        Ok((wit_config, components))
    }

    /// Parsers the component located at `path` and creates all information
    /// necessary for a `Component` return value.
    fn parse_component(&self, path: &Path, kind: Kind, mut bindgen: Bindgen) -> Result<Component> {
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .context("non-utf-8 path extension")?;

        let mut language = match extension {
            "rs" => Language::Rust,
            "c" => Language::C,
            "cpp" => Language::Cpp17,
            "wat" => Language::Wat,
            "cs" => Language::Csharp,
            "mbt" => Language::MoonBit,
            other => Language::Custom(custom::Language::lookup(self, other)?),
        };

        let contents = fs::read_to_string(&path)?;
        let config = match language.obj().comment_prefix_for_test_config() {
            Some(comment) => {
                config::parse_test_config::<config::RuntimeTestConfig>(&contents, comment)?
            }
            None => Default::default(),
        };
        assert!(bindgen.args.is_empty());
        bindgen.args = config.args.into();
        if language == Language::Cpp17 {
            bindgen.args.retain(|elem| {
                if elem == "--language=Cpp" {
                    language = Language::Cpp;
                    false
                } else {
                    true
                }
            });
        }

        let has_link_name = bindgen
            .args
            .iter()
            .any(|elem| elem.starts_with("--link-name"));
        if self.is_symmetric() && matches!(kind, Kind::Runner) && !has_link_name {
            match &language {
                Language::Rust => {
                    bindgen.args.push(String::from("--link-name"));
                    bindgen.args.push(String::from("test-rust"));
                }
                _ => {
                    println!("Symmetric: --link_name missing from language {language:?}");
                    // todo!();
                }
            }
        }

        Ok(Component {
            name: path.file_stem().unwrap().to_str().unwrap().to_string(),
            path: path.to_path_buf(),
            language,
            bindgen,
            kind,
            contents,
            lang_config: config.lang,
        })
    }

    /// Prepares all languages in use in `test` as part of a one-time
    /// initialization step.
    fn prepare_languages(&mut self, tests: &HashMap<String, Test>) -> Result<()> {
        let all_languages = self.all_languages();

        let mut prepared = HashSet::new();
        let mut prepare = |lang: &Language, name: &str| -> Result<()> {
            if !self.include_language(lang) || !prepared.insert(lang.clone()) {
                return Ok(());
            }
            lang.obj()
                .prepare(self, name)
                .with_context(|| format!("failed to prepare language {lang}"))
        };

        for test in tests.values() {
            match &test.kind {
                TestKind::Runtime(c) => {
                    for component in c {
                        prepare(&component.language, &test.name)?
                    }
                }
                TestKind::Codegen(_) => {
                    for lang in all_languages.iter() {
                        prepare(lang, &test.name)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn all_languages(&self) -> Vec<Language> {
        let mut languages = Language::ALL.to_vec();
        for (ext, _) in self.opts.custom.custom.iter() {
            languages.push(Language::Custom(
                custom::Language::lookup(self, ext).unwrap(),
            ));
        }
        languages
    }

    /// Executes all tests that are `TestKind::Codegen`.
    fn run_codegen_tests(&mut self, tests: &HashMap<String, Test>) -> Result<()> {
        let mut codegen_tests = Vec::new();
        let languages = self.all_languages();
        for (name, config, test) in tests.iter().filter_map(|(name, t)| match &t.kind {
            TestKind::Runtime(_) => None,
            TestKind::Codegen(p) => Some((name, &t.config, p)),
        }) {
            if let Some(filter) = &self.opts.filter {
                if !filter.is_match(name) {
                    continue;
                }
            }
            for language in languages.iter() {
                // Right now C++'s generator is the same as C's, so don't
                // duplicate everything there.
                if *language == Language::Cpp {
                    continue;
                }

                // If the CLI arguments filter out this language, then discard
                // the test case.
                if !self.include_language(&language) {
                    continue;
                }

                let mut args = Vec::new();
                for arg in language.obj().default_bindgen_args_for_codegen() {
                    args.push(arg.to_string());
                }

                if self.is_symmetric() {
                    args.push(String::from("--symmetric"))
                }

                codegen_tests.push((
                    language.clone(),
                    test,
                    name.to_string(),
                    args.clone(),
                    config.clone(),
                ));

                for (args_kind, new_args) in language.obj().codegen_test_variants() {
                    let mut args = args.clone();
                    for arg in new_args.iter() {
                        args.push(arg.to_string());
                    }
                    codegen_tests.push((
                        language.clone(),
                        test,
                        format!("{name}-{args_kind}"),
                        args,
                        config.clone(),
                    ));
                }
            }
        }

        if codegen_tests.is_empty() {
            return Ok(());
        }

        println!("Running {} codegen tests:", codegen_tests.len());

        let results = codegen_tests
            .par_iter()
            .map(|(language, test, args_kind, args, config)| {
                let should_fail = language.obj().should_fail_verify(args_kind, config, args);
                let result = self
                    .codegen_test(language, test, &args_kind, args, config)
                    .with_context(|| {
                        format!("failed to codegen test for `{language}` over {test:?}")
                    });
                self.update_status(&result, should_fail);
                (result, should_fail, language, test, args_kind)
            })
            .collect::<Vec<_>>();

        println!("");

        self.render_errors(results.into_iter().map(
            |(result, should_fail, language, test, args_kind)| {
                StepResult::new(test.to_str().unwrap(), result)
                    .should_fail(should_fail)
                    .metadata("language", language)
                    .metadata("variant", args_kind)
            },
        ));

        Ok(())
    }

    /// Runs a single codegen test.
    ///
    /// This will generate bindings for `test` in the `language` specified. The
    /// test name is mangled by `args_kind` and the `args` are arguments to pass
    /// to the bindings generator.
    fn codegen_test(
        &self,
        language: &Language,
        test: &Path,
        args_kind: &str,
        args: &[String],
        config: &config::WitConfig,
    ) -> Result<()> {
        let mut resolve = wit_parser::Resolve::default();
        let (pkg, _) = resolve.push_path(test).context("failed to load WIT")?;
        let world = resolve
            .select_world(pkg, None)
            .or_else(|err| resolve.select_world(pkg, Some("imports")).map_err(|_| err))
            .context("failed to select a world for bindings generation")?;
        let world = resolve.worlds[world].name.clone();

        let artifacts_dir = std::env::current_dir()?
            .join(&self.opts.artifacts)
            .join("codegen")
            .join(language.to_string())
            .join(args_kind);
        let _ = fs::remove_dir_all(&artifacts_dir);
        let bindings_dir = artifacts_dir.join("bindings");
        let bindgen = Bindgen {
            args: args.to_vec(),
            wit_path: test.to_path_buf(),
            world: world.clone(),
            wit_config: config.clone(),
        };
        language
            .obj()
            .generate_bindings(self, &bindgen, &bindings_dir)
            .context("failed to generate bindings")?;

        language
            .obj()
            .verify(
                self,
                &Verify {
                    world: &world,
                    artifacts_dir: &artifacts_dir,
                    bindings_dir: &bindings_dir,
                    wit_test: test,
                    args: &bindgen.args,
                },
            )
            .context("failed to verify generated bindings")?;

        Ok(())
    }

    /// Execute all `TestKind::Runtime` tests
    fn run_runtime_tests(&mut self, tests: &HashMap<String, Test>) -> Result<()> {
        let components = tests
            .values()
            .filter(|t| match &self.opts.filter {
                Some(filter) => filter.is_match(&t.name),
                None => true,
            })
            .filter_map(|t| match &t.kind {
                TestKind::Runtime(c) => Some(c.iter().map(move |c| (t, c))),
                TestKind::Codegen(_) => None,
            })
            .flat_map(|i| i)
            // Discard components that are unrelated to the languages being
            // tested.
            .filter(|(_test, component)| self.include_language(&component.language))
            .collect::<Vec<_>>();

        println!("Compiling {} components:", components.len());

        // In parallel compile all sources to their binary component
        // form.
        let compile_results = components
            .iter()
            .map(|(test, component)| {
                let path = self
                    .compile_component(test, component)
                    .with_context(|| format!("failed to compile component {:?}", component.path));
                self.update_status(&path, false);
                (test, component, path)
            })
            .collect::<Vec<_>>();
        println!("");

        let mut compilations = Vec::new();
        self.render_errors(
            compile_results
                .into_iter()
                .map(|(test, component, result)| match result {
                    Ok(path) => {
                        compilations.push((test, component, path));
                        StepResult::new("", Ok(()))
                    }
                    Err(e) => StepResult::new(&test.name, Err(e))
                        .metadata("component", &component.name)
                        .metadata("path", component.path.display()),
                }),
        );

        // Next, massage the data a bit. Create a map of all tests to where
        // their components are located. Then perform a product of runners/tests
        // to generate a list of test cases. Finally actually execute the testj
        // cases.
        let mut compiled_components = HashMap::new();
        for (test, component, path) in compilations {
            let list = compiled_components.entry(&test.name).or_insert(Vec::new());
            list.push((*component, path));
        }

        let mut to_run = Vec::new();
        for (test, components) in compiled_components.iter() {
            for a in components.iter().filter(|(c, _)| c.kind == Kind::Runner) {
                self.push_tests(&tests[test.as_str()], components, a, &mut to_run)?;
            }
        }

        println!("Running {} runtime tests:", to_run.len());

        let results = to_run
            .par_iter()
            .map(|(case_name, (runner, runner_path), test_components)| {
                let case = &tests[*case_name];
                let result = self
                    .runtime_test(case, runner, runner_path, test_components)
                    .with_context(|| format!("failed to run `{}`", case.name));
                self.update_status(&result, false);
                (result, case_name, runner, runner_path, test_components)
            })
            .collect::<Vec<_>>();

        println!("");

        self.render_errors(results.into_iter().map(
            |(result, case_name, runner, runner_path, test_components)| {
                let mut result = StepResult::new(case_name, result)
                    .metadata("runner", runner.path.display())
                    .metadata("compiled runner", runner_path.display());
                for (test, path) in test_components {
                    result = result
                        .metadata("test", test.path.display())
                        .metadata("compiled test", path.display());
                }
                result
            },
        ));

        Ok(())
    }

    /// For the `test` provided, and the selected `runner`, determines all
    /// permutations of tests from `components` and pushes them on to `to_run`.
    fn push_tests<'a>(
        &self,
        test: &'a Test,
        components: &'a [(&'a Component, PathBuf)],
        runner: &'a (&'a Component, PathBuf),
        to_run: &mut Vec<(
            &'a str,
            (&'a Component, &'a Path),
            Vec<(&'a Component, &'a Path)>,
        )>,
    ) -> Result<()> {
        /// Recursive function which walks over `worlds`, the list of worlds
        /// that `test` expects, one by one. For each world it finds a matching
        /// component in `components` adn then recurses for the next item in the
        /// `worlds` list.
        ///
        /// Once `worlds` is empty the `test` list, a temporary vector, is
        /// cloned and pushed into `commit`.
        fn push<'a>(
            worlds: &[String],
            components: &'a [(&'a Component, PathBuf)],
            test: &mut Vec<(&'a Component, &'a Path)>,
            commit: &mut dyn FnMut(Vec<(&'a Component, &'a Path)>),
        ) -> Result<()> {
            match worlds.split_first() {
                Some((world, rest)) => {
                    let mut any = false;
                    for (component, path) in components {
                        if component.bindgen.world == *world {
                            any = true;
                            test.push((component, path));
                            push(rest, components, test, commit)?;
                            test.pop();
                        }
                    }
                    if !any {
                        bail!("no components found for `{world}`");
                    }
                }

                // No more `worlds`? Then `test` is our set of test components.
                None => commit(test.clone()),
            }
            Ok(())
        }

        push(
            &test.config.dependency_worlds(),
            components,
            &mut Vec::new(),
            &mut |test_components| {
                to_run.push((&test.name, (runner.0, &runner.1), test_components));
            },
        )
    }

    /// Compiles the `component` specified to wasm for the `test` given.
    ///
    /// This will generate bindings for `component` and then perform
    /// language-specific compilation to convert the files into a component.
    fn compile_component(&self, test: &Test, component: &Component) -> Result<PathBuf> {
        let root_dir = std::env::current_dir()?
            .join(&self.opts.artifacts)
            .join(&test.name);
        let artifacts_dir = root_dir.join(format!("{}-{}", component.name, component.language));
        let _ = fs::remove_dir_all(&artifacts_dir);
        let bindings_dir = artifacts_dir.join("bindings");
        let output = root_dir.join(if self.is_symmetric() {
            match &component.kind {
                Kind::Runner => format!("{}-{}_exe", component.name, component.language),
                Kind::Test => format!("lib{}-{}.so", component.name, component.language),
            }
        } else {
            format!("{}-{}.wasm", component.name, component.language)
        });
        component
            .language
            .obj()
            .generate_bindings(self, &component.bindgen, &bindings_dir)?;
        let result = Compile {
            component,
            bindings_dir: &bindings_dir,
            artifacts_dir: &artifacts_dir,
            output: &output,
        };
        component.language.obj().compile(self, &result)?;

        if !self.is_symmetric() {
            // Double-check the output is indeed a component and it's indeed valid.
            let wasm = fs::read(&output)
                .with_context(|| format!("failed to read output wasm file {output:?}"))?;
            if !wasmparser::Parser::is_component(&wasm) {
                bail!("output file {output:?} is not a component");
            }
            wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::all())
                .validate_all(&wasm)
                .with_context(|| format!("compiler produced invalid wasm file {output:?}"))?;
        }

        Ok(output)
    }

    /// Executes a single test case.
    ///
    /// Composes `runner_wasm` with the components in `test_components` and then
    /// executes it with the runner specified in CLI flags.
    fn runtime_test(
        &self,
        case: &Test,
        runner: &Component,
        runner_wasm: &Path,
        test_components: &[(&Component, &Path)],
    ) -> Result<()> {
        // If possible use `wasm-compose` to compose the test together. This is
        // only possible when customization isn't used though. This is also only
        // done for async tests at this time to ensure that there's a version of
        // composition that's done which is at the same version as wasmparser
        // and friends.
        let composed = if self.is_symmetric() {
            Vec::new()
        } else if case.config.wac.is_none() && test_components.len() == 1 {
            self.compose_wasm_with_wasm_compose(runner_wasm, test_components)?
        } else {
            self.compose_wasm_with_wac(case, runner, runner_wasm, test_components)?
        };

        let dst = runner_wasm.parent().unwrap();
        let mut filename = format!(
            "composed-{}",
            runner.path.file_name().unwrap().to_str().unwrap(),
        );
        for (test, _) in test_components {
            filename.push_str("-");
            filename.push_str(test.path.file_name().unwrap().to_str().unwrap());
        }
        if !self.is_symmetric() {
            filename.push_str(".wasm");
        }
        let composed_wasm = dst.join(filename);
        if !self.is_symmetric() {
            write_if_different(&composed_wasm, &composed)?;

            self.run_command(self.test_runner.command().arg(&composed_wasm))?;
        } else {
            if std::fs::exists(composed_wasm.clone())? {
                std::fs::remove_dir_all(composed_wasm.clone())?;
            }
            std::fs::create_dir(composed_wasm.clone())?;

            let mut new_file = composed_wasm.clone();
            new_file.push(&(runner_wasm.file_name().unwrap()));
            symlink(runner_wasm, new_file)?;
            for (_c, p) in test_components.iter() {
                let mut new_file = composed_wasm.clone();
                new_file.push(&(p.file_name().unwrap()));
                symlink(p, new_file)?;
            }
            let cwd = runner_wasm.parent().unwrap().parent().unwrap();
            let dir = cwd.join("rust");
            let wit_bindgen = dir.join("wit-bindgen");
            let so_dir = wit_bindgen.join("target").join("debug").join("deps");
            symlink(
                so_dir.join("libsymmetric_executor.so"),
                composed_wasm.join("libsymmetric_executor.so"),
            )?;
            symlink(
                so_dir.join("libsymmetric_stream.so"),
                composed_wasm.join("libsymmetric_stream.so"),
            )?;

            let mut cmd = Command::new(runner_wasm);
            cmd.env("LD_LIBRARY_PATH", ".");
            cmd.current_dir(composed_wasm);
            self.run_command(&mut cmd)?;
        }
        Ok(())
    }

    fn compose_wasm_with_wasm_compose(
        &self,
        runner_wasm: &Path,
        test_components: &[(&Component, &Path)],
    ) -> Result<Vec<u8>> {
        assert!(test_components.len() == 1);
        let test_wasm = test_components[0].1;
        let mut config = wasm_compose::config::Config::default();
        config.definitions = vec![test_wasm.to_path_buf()];
        wasm_compose::composer::ComponentComposer::new(runner_wasm, &config)
            .compose()
            .with_context(|| format!("failed to compose {runner_wasm:?} with {test_wasm:?}"))
    }

    fn compose_wasm_with_wac(
        &self,
        case: &Test,
        runner: &Component,
        runner_wasm: &Path,
        test_components: &[(&Component, &Path)],
    ) -> Result<Vec<u8>> {
        let document = match &case.config.wac {
            Some(path) => {
                let wac_config = case.path.join(path);
                fs::read_to_string(&wac_config)
                    .with_context(|| format!("failed to read {wac_config:?}"))?
            }
            // Default wac script is to just make `test_components` available
            // to the `runner`.
            None => {
                let mut script = String::from("package example:composition;\n");
                let mut args = Vec::new();
                for (component, _path) in test_components {
                    let world = &component.bindgen.world;
                    args.push(format!("...{world}"));
                    script.push_str(&format!("let {world} = new test:{world} {{ ... }};\n"));
                }
                args.push("...".to_string());
                let runner = &runner.bindgen.world;
                script.push_str(&format!(
                    "let runner = new test:{runner} {{ {} }};\n\
                     export runner...;",
                    args.join(", ")
                ));

                script
            }
        };

        // Get allocations for `test:{world}` rooted on the stack as
        // `BorrowedPackageKey` below requires `&str`.
        let components_as_packages = test_components
            .iter()
            .map(|(component, path)| {
                Ok((format!("test:{}", component.bindgen.world), fs::read(path)?))
            })
            .collect::<Result<Vec<_>>>()?;

        let runner_name = format!("test:{}", runner.bindgen.world);
        let mut packages = indexmap::IndexMap::new();
        packages.insert(
            wac_types::BorrowedPackageKey {
                name: &runner_name,
                version: None,
            },
            fs::read(runner_wasm)?,
        );
        for (name, contents) in components_as_packages.iter() {
            packages.insert(
                wac_types::BorrowedPackageKey {
                    name,
                    version: None,
                },
                contents.clone(),
            );
        }

        // TODO: should figure out how to render these errors better.
        let document =
            wac_parser::Document::parse(&document).context("failed to parse wac script")?;
        document
            .resolve(packages)
            .context("failed to run `wac` resolve")?
            .encode(wac_graph::EncodeOptions {
                define_components: true,
                validate: false,
                processor: None,
            })
            .context("failed to encode `wac` result")
    }

    /// Helper to execute an external process and generate a helpful error
    /// message on failure.
    fn run_command(&self, cmd: &mut Command) -> Result<()> {
        if self.opts.inherit_stderr {
            cmd.stderr(Stdio::inherit());
        }
        let output = cmd
            .output()
            .with_context(|| format!("failed to spawn {cmd:?}"))?;
        if output.status.success() {
            if std::env::var("WIT_BINDGEN_TRACE").is_ok() {
                eprintln!("$ {cmd:?}");
            }
            return Ok(());
        }

        let mut error = format!(
            "\
command execution failed
command: {cmd:?}
status: {}",
            output.status,
        );

        if !output.stdout.is_empty() {
            error.push_str(&format!(
                "\nstdout:\n  {}",
                String::from_utf8_lossy(&output.stdout).replace("\n", "\n  ")
            ));
        }
        if !output.stderr.is_empty() {
            error.push_str(&format!(
                "\nstderr:\n  {}",
                String::from_utf8_lossy(&output.stderr).replace("\n", "\n  ")
            ));
        }

        bail!("{error}")
    }

    /// Converts the WASIp1 module at `p1` to a component using the information
    /// stored within `compile`.
    ///
    /// Stores the output at `compile.output`.
    fn convert_p1_to_component(&self, p1: &Path, compile: &Compile<'_>) -> Result<()> {
        let mut resolve = wit_parser::Resolve::default();
        let (pkg, _) = resolve
            .push_path(&compile.component.bindgen.wit_path)
            .context("failed to load WIT")?;
        let world = resolve.select_world(pkg, Some(&compile.component.bindgen.world))?;
        let mut module = fs::read(&p1).context("failed to read wasm file")?;
        let encoded = wit_component::metadata::encode(&resolve, world, StringEncoding::UTF8, None)?;

        let section = wasm_encoder::CustomSection {
            name: Cow::Borrowed("component-type"),
            data: Cow::Borrowed(&encoded),
        };
        module.push(section.id());
        section.encode(&mut module);

        let wasi_adapter = match compile.component.kind {
            Kind::Runner => {
                wasi_preview1_component_adapter_provider::WASI_SNAPSHOT_PREVIEW1_COMMAND_ADAPTER
            }
            Kind::Test => {
                wasi_preview1_component_adapter_provider::WASI_SNAPSHOT_PREVIEW1_REACTOR_ADAPTER
            }
        };

        let component = ComponentEncoder::default()
            .module(module.as_slice())
            .context("failed to load custom sections from input module")?
            .validate(true)
            .adapter("wasi_snapshot_preview1", wasi_adapter)
            .context("failed to load wasip1 adapter")?
            .encode()
            .context("failed to convert to a component")?;
        write_if_different(compile.output, component)?;
        Ok(())
    }

    /// "poor man's test output progress"
    fn update_status<T>(&self, result: &Result<T>, should_fail: bool) {
        if result.is_ok() == !should_fail {
            print!(".");
        } else {
            print!("F");
        }
        let _ = std::io::stdout().flush();
    }

    /// Returns whether `languages` is included in this testing session.
    fn include_language(&self, language: &Language) -> bool {
        self.opts
            .languages
            .iter()
            .any(|l| l == language.obj().display())
    }

    fn render_errors<'a>(&self, results: impl Iterator<Item = StepResult<'a>>) {
        let mut failures = 0;
        for result in results {
            let err = match (result.result, result.should_fail) {
                (Ok(()), false) | (Err(_), true) => continue,
                (Err(e), false) => e,
                (Ok(()), true) => anyhow!("test should have failed, but passed"),
            };
            failures += 1;

            println!("------ Failure: {} --------", result.name);
            for (k, v) in result.metadata {
                println!("  {k}: {v}");
            }
            println!("  error: {}", format!("{err:?}").replace("\n", "\n  "));
        }

        if failures > 0 {
            println!("{failures} tests FAILED");
            std::process::exit(1);
        }
    }

    fn is_symmetric(&self) -> bool {
        self.opts.symmetric
    }
}

struct StepResult<'a> {
    result: Result<()>,
    should_fail: bool,
    name: &'a str,
    metadata: Vec<(&'a str, String)>,
}

impl<'a> StepResult<'a> {
    fn new(name: &'a str, result: Result<()>) -> StepResult<'a> {
        StepResult {
            name,
            result,
            should_fail: false,
            metadata: Vec::new(),
        }
    }

    fn should_fail(mut self, fail: bool) -> Self {
        self.should_fail = fail;
        self
    }

    fn metadata(mut self, name: &'a str, value: impl fmt::Display) -> Self {
        self.metadata.push((name, value.to_string()));
        self
    }
}

/// Helper trait for each language to implement which encapsulates
/// language-specific logic.
trait LanguageMethods {
    /// Display name for this language, used in filenames.
    fn display(&self) -> &str;

    /// Returns the prefix that this language uses to annotate configuration in
    /// the top of source files.
    ///
    /// This should be the language's line-comment syntax followed by `@`, e.g.
    /// `//@` for Rust or `;;@` for WebAssembly Text.
    fn comment_prefix_for_test_config(&self) -> Option<&str>;

    /// Returns the extra permutations, if any, of arguments to use with codegen
    /// tests.
    ///
    /// This is used to run all codegen tests with a variety of bindings
    /// generator options. The first element in the tuple is a descriptive
    /// string that should be unique (used in file names) and the second elemtn
    /// is the list of arguments for that variant to pass to the bindings
    /// generator.
    fn codegen_test_variants(&self) -> &[(&str, &[&str])] {
        &[]
    }

    /// Performs any one-time preparation necessary for this language, such as
    /// downloading or caching dependencies.
    fn prepare(&self, runner: &mut Runner<'_>, name: &str) -> Result<()>;

    /// Add some files to the generated directory _before_ calling bindgen
    fn generate_bindings_prepare(
        &self,
        _runner: &Runner<'_>,
        _bindgen: &Bindgen,
        _dir: &Path,
    ) -> Result<()> {
        Ok(())
    }

    /// Generates bindings for `component` into `dir`.
    ///
    /// Runs `wit-bindgen` in aa subprocess to catch failures such as panics.
    fn generate_bindings(&self, runner: &Runner<'_>, bindgen: &Bindgen, dir: &Path) -> Result<()> {
        let name = match self.bindgen_name() {
            Some(name) => name,
            None => return Ok(()),
        };
        self.generate_bindings_prepare(runner, bindgen, dir)?;
        let mut cmd = Command::new(runner.wit_bindgen);
        cmd.arg(name)
            .arg(&bindgen.wit_path)
            .arg("--world")
            .arg(format!("%{}", bindgen.world))
            .arg("--out-dir")
            .arg(dir);
        if runner.is_symmetric() {
            cmd.arg("--symmetric");
        }

        match bindgen.wit_config.default_bindgen_args {
            Some(true) | None => {
                for arg in self.default_bindgen_args() {
                    cmd.arg(arg);
                }
            }
            Some(false) => {}
        }

        for arg in bindgen.args.iter() {
            cmd.arg(arg);
        }

        runner.run_command(&mut cmd)
    }

    /// Returns the default set of arguments that will be passed to
    /// `wit-bindgen`.
    ///
    /// Defaults to empty, but each language can override it.
    fn default_bindgen_args(&self) -> &[&str] {
        &[]
    }

    /// Same as `default_bindgen_args` but specifically applied during codegen
    /// tests, such as generating stub impls by default.
    fn default_bindgen_args_for_codegen(&self) -> &[&str] {
        &[]
    }

    /// Returns the name of this bindings generator when passed to
    /// `wit-bindgen`.
    ///
    /// By default this is `Some(self.display())`, but it can be overridden if
    /// necessary. Returning `None` here means that no bindings generator is
    /// supported.
    fn bindgen_name(&self) -> Option<&str> {
        Some(self.display())
    }

    /// Performs compilation as specified by `compile`.
    fn compile(&self, runner: &Runner<'_>, compile: &Compile) -> Result<()>;

    /// Returns whether this language is supposed to fail this codegen tests
    /// given the `config` and `args` for the test.
    fn should_fail_verify(&self, name: &str, config: &config::WitConfig, args: &[String]) -> bool;

    /// Performs a "check" or a verify that the generated bindings described by
    /// `Verify` are indeed valid.
    fn verify(&self, runner: &Runner<'_>, verify: &Verify) -> Result<()>;
}

impl Language {
    const ALL: &[Language] = &[
        Language::Rust,
        Language::C,
        Language::Cpp,
        Language::Cpp17,
        Language::Wat,
        Language::Csharp,
        Language::MoonBit,
    ];

    fn obj(&self) -> &dyn LanguageMethods {
        match self {
            Language::Rust => &rust::Rust,
            Language::C => &c::C,
            Language::Cpp => &c::Cpp,
            Language::Cpp17 => &cpp::Cpp17,
            Language::Wat => &wat::Wat,
            Language::Csharp => &csharp::Csharp,
            Language::MoonBit => &moonbit::MoonBit,
            Language::Custom(custom) => custom,
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.obj().display().fmt(f)
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Runner => "runner".fmt(f),
            Kind::Test => "test".fmt(f),
        }
    }
}

/// Returns `true` if the file was written, or `false` if the file is the same
/// as it was already on disk.
fn write_if_different(path: &Path, contents: impl AsRef<[u8]>) -> Result<bool> {
    let contents = contents.as_ref();
    if let Ok(prev) = fs::read(path) {
        if prev == contents {
            return Ok(false);
        }
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {parent:?}"))?;
    }
    fs::write(path, contents).with_context(|| format!("failed to write {path:?}"))?;
    Ok(true)
}

impl Component {
    /// Helper to convert `RuntimeTestConfig` to a `RuntimeTestConfig<T>` and
    /// then extract the `T`.
    ///
    /// This is called from within each language's implementation with a
    /// specific `T` necessary for that language.
    fn deserialize_lang_config<T>(&self) -> Result<T>
    where
        T: Default + serde::de::DeserializeOwned,
    {
        // If this test has no language-specific configuration then return this
        // language's default configuration.
        if self.lang_config.is_none() {
            return Ok(T::default());
        }

        // Otherwise re-parse the TOML at the top of the file but this time
        // with the specific `T` that we're interested in. This is expected
        // to then produce a value in the `lang` field since
        // `self.lang_config.is_some()` is true.
        let config = config::parse_test_config::<config::RuntimeTestConfig<T>>(
            &self.contents,
            self.language
                .obj()
                .comment_prefix_for_test_config()
                .unwrap(),
        )?;
        Ok(config.lang.unwrap())
    }
}
