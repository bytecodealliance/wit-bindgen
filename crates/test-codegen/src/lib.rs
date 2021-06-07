use proc_macro::TokenStream;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use witx_bindgen_gen_core::Generator;

#[proc_macro]
pub fn test_rust_codegen(_input: TokenStream) -> TokenStream {
    generate_tests("crates/gen-rust-wasm/tests".as_ref(), |path| {
        let mut opts = witx_bindgen_gen_rust_wasm::Opts::default();
        opts.rustfmt = true;
        opts.unchecked = path.to_str().unwrap().contains("unchecked");
        opts.build()
    })
}

fn generate_tests<G>(root: &Path, mkgen: impl Fn(&Path) -> G) -> TokenStream
where
    G: Generator,
{
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));
    let mut ret = TokenStream::new();
    for test in find_tests(root) {
        let mut gen = mkgen(&test);
        let mut files = Default::default();
        let iface = witx2::Interface::parse_file(&test).unwrap();
        gen.generate(
            &iface,
            !test.to_str().unwrap().contains("export"),
            &mut files,
        );

        let dst = out_dir.join(test.file_stem().unwrap());
        drop(fs::remove_dir_all(&dst));
        fs::create_dir_all(&dst).unwrap();
        for (file, contents) in files.iter() {
            fs::write(dst.join(file), contents).unwrap();
        }
        ret.extend(
            format!("include!(\"{}\");", dst.join("bindings.rs").display())
                .parse::<TokenStream>()
                .unwrap(),
        );
    }
    ret
}

fn find_tests(root: &Path) -> Vec<PathBuf> {
    let mut tests = Vec::new();
    find_tests(root, &mut tests);
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
