//! You can run this test suite with:
//!
//!     cargo test --test all
//!
//! An argument can be passed as well to filter, based on filename, which test
//! to run
//!
//!     cargo test --test all foo.witx

use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use serde::Serialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

fn main() {
    let tests = find_tests();
    let filter = std::env::args().nth(1);

    let tests = tests
        .par_iter()
        .filter_map(|test| {
            if let Some(filter) = &filter {
                if let Some(s) = test.to_str() {
                    if !s.contains(filter) {
                        return None;
                    }
                }
            }
            let contents = fs::read(test).unwrap();
            Some((test, contents))
        })
        .collect::<Vec<_>>();

    println!("running {} test files\n", tests.len());

    let ntests = AtomicUsize::new(0);
    let errors = tests
        .par_iter()
        .filter_map(|(test, contents)| {
            Runner { ntests: &ntests }
                .run(test, contents)
                .context(format!("test {:?} failed", test))
                .err()
        })
        .collect::<Vec<_>>();

    if !errors.is_empty() {
        for msg in errors.iter() {
            eprintln!("{:?}", msg);
        }

        panic!("{} tests failed", errors.len())
    }

    println!(
        "test result: ok. {} directives passed\n",
        ntests.load(SeqCst)
    );
}

/// Recursively finds all tests in a whitelisted set of directories which we
/// then load up and test in parallel.
fn find_tests() -> Vec<PathBuf> {
    let mut tests = Vec::new();
    find_tests("tests/ui".as_ref(), &mut tests);
    tests.sort();
    return tests;

    fn find_tests(path: &Path, tests: &mut Vec<PathBuf>) {
        for f in path.read_dir().unwrap() {
            let f = f.unwrap();
            if f.file_type().unwrap().is_dir() {
                find_tests(&f.path(), tests);
                continue;
            }

            match f.path().extension().and_then(|s| s.to_str()) {
                Some("witx") => {}
                _ => continue,
            }
            tests.push(f.path());
        }
    }
}

struct Runner<'a> {
    ntests: &'a AtomicUsize,
}

impl Runner<'_> {
    fn run(&mut self, test: &Path, contents: &[u8]) -> Result<()> {
        let contents = str::from_utf8(contents)?;

        let result = witx2::Instance::parse_file(test);

        let result = if contents.contains("// parse-fail") {
            match result {
                Ok(_) => bail!("expected test to not parse but it did"),
                Err(e) => format!("{:?}", e),
            }
        } else {
            let instance = result?;
            to_json(&instance)
        };

        let result_file = test.with_extension("witx.result");
        if env::var_os("BLESS").is_some() {
            fs::write(&result_file, result)?;
        } else {
            let expected = fs::read_to_string(&result_file).context(format!(
                "failed to read test expectation file {:?}\nthis can be fixed with BLESS=1",
                result_file
            ))?;
            if expected != result {
                bail!("failed test");
            }
        }
        self.bump_ntests();
        Ok(())
    }

    fn bump_ntests(&self) {
        self.ntests.fetch_add(1, SeqCst);
    }
}

fn to_json(i: &witx2::Instance) -> String {
    #[derive(Serialize)]
    struct Instance {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        resources: Vec<Resource>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        types: Vec<TypeDef>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        functions: Vec<Function>,
    }

    #[derive(Serialize)]
    struct Resource {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        foreign_module: Option<String>,
    }

    #[derive(Serialize)]
    struct TypeDef {
        idx: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(flatten)]
        ty: Type,
        #[serde(skip_serializing_if = "Option::is_none")]
        foreign_module: Option<String>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "kebab-case")]
    enum Type {
        Primitive(String),
        Record {
            fields: Vec<(String, String)>,
        },
        Variant {
            cases: Vec<(String, Option<String>)>,
        },
        List(String),
        PushBuffer(String),
        PullBuffer(String),
    }

    #[derive(Serialize)]
    struct Function {
        name: String,
        params: Vec<String>,
        results: Vec<String>,
    }

    let resources = i
        .resources
        .iter()
        .map(|(_, r)| Resource {
            name: r.name.clone(),
            foreign_module: r.foreign_module.clone(),
        })
        .collect::<Vec<_>>();

    let types = i
        .types
        .iter()
        .map(|(i, r)| TypeDef {
            idx: i.index(),
            name: r.name.clone(),
            ty: translate_typedef(r),
            foreign_module: r.foreign_module.clone(),
        })
        .collect::<Vec<_>>();
    let functions = i
        .functions
        .iter()
        .map(|f| Function {
            name: f.name.clone(),
            params: f.params.iter().map(|(_, ty)| translate_type(ty)).collect(),
            results: f.results.iter().map(translate_type).collect(),
        })
        .collect::<Vec<_>>();

    let instance = Instance {
        resources,
        types,
        functions,
    };
    return serde_json::to_string_pretty(&instance).unwrap();

    fn translate_typedef(ty: &witx2::TypeDef) -> Type {
        match &ty.kind {
            witx2::TypeDefKind::Type(t) => Type::Primitive(translate_type(t)),
            witx2::TypeDefKind::Record(r) => Type::Record {
                fields: r
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), translate_type(&f.ty)))
                    .collect(),
            },
            witx2::TypeDefKind::Variant(v) => Type::Variant {
                cases: v
                    .cases
                    .iter()
                    .map(|f| (f.name.clone(), f.ty.as_ref().map(translate_type)))
                    .collect(),
            },
            witx2::TypeDefKind::PushBuffer(ty) => Type::PushBuffer(translate_type(ty)),
            witx2::TypeDefKind::PullBuffer(ty) => Type::PullBuffer(translate_type(ty)),
            witx2::TypeDefKind::List(ty) => Type::List(translate_type(ty)),
        }
    }

    fn translate_type(ty: &witx2::Type) -> String {
        match ty {
            witx2::Type::U8 => format!("u8"),
            witx2::Type::U16 => format!("u16"),
            witx2::Type::U32 => format!("u32"),
            witx2::Type::U64 => format!("u64"),
            witx2::Type::S8 => format!("s8"),
            witx2::Type::S16 => format!("s16"),
            witx2::Type::S32 => format!("s32"),
            witx2::Type::S64 => format!("s64"),
            witx2::Type::F32 => format!("f32"),
            witx2::Type::F64 => format!("f64"),
            witx2::Type::Char => format!("char"),
            witx2::Type::Handle(resource) => format!("handle-{}", resource.index()),
            witx2::Type::Id(id) => format!("type-{}", id.index()),
        }
    }
}
