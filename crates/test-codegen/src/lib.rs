use ignore::gitignore::GitignoreBuilder;
use proc_macro::{TokenStream, TokenTree};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use witx_bindgen_gen_core::Generator;

#[proc_macro]
#[cfg(feature = "witx-bindgen-gen-rust-wasm")]
pub fn rust_wasm_import(input: TokenStream) -> TokenStream {
    let checked = generate_tests(input.clone(), "import-checked", |_path| {
        (witx_bindgen_gen_rust_wasm::Opts::default().build(), true)
    });
    let checked = include_rust_sources(checked.iter().map(|p| p.1.join("bindings.rs")));
    let unchecked = generate_tests(input, "import-unchecked", |_path| {
        let mut opts = witx_bindgen_gen_rust_wasm::Opts::default();
        opts.unchecked = true;
        (opts.build(), true)
    });
    let unchecked = include_rust_sources(unchecked.iter().map(|p| p.1.join("bindings.rs")));

    return quote::quote! {
        mod checked { #checked }
        mod unchecked { #unchecked }
    }
    .into();
}

#[proc_macro]
#[cfg(feature = "witx-bindgen-gen-rust-wasm")]
pub fn rust_wasm_export(input: TokenStream) -> TokenStream {
    use heck::*;

    let checked = generate_tests(input.clone(), "export-checked", |_path| {
        (witx_bindgen_gen_rust_wasm::Opts::default().build(), false)
    });
    let checked_extra = checked.iter().map(|p| gen_extra(&p.0));
    let checked = include_rust_sources(checked.iter().map(|p| p.1.join("bindings.rs")));

    let unchecked = generate_tests(input, "export-unchecked", |_path| {
        let mut opts = witx_bindgen_gen_rust_wasm::Opts::default();
        opts.unchecked = true;
        opts.symbol_namespace = "unchecked".to_string();
        (opts.build(), false)
    });
    let unchecked_extra = unchecked.iter().map(|p| gen_extra(&p.0));
    let unchecked = include_rust_sources(unchecked.iter().map(|p| p.1.join("bindings.rs")));

    return quote::quote! {
        mod checked {
            #checked
            #(#checked_extra)*
        }
        mod unchecked {
            #unchecked
            #(#unchecked_extra)*
        }
    }
    .into();

    fn gen_extra(iface: &witx2::Interface) -> proc_macro2::TokenStream {
        if iface.functions.len() == 0 {
            return quote::quote! {};
        }

        let methods = iface.functions.iter().map(|f| {
            let name = quote::format_ident!("{}", f.name.to_snake_case());
            let params = f.params.iter().map(|(_, t)| quote_ty(iface, t));
            let mut results = f.results.iter().map(|(_, t)| quote_ty(iface, t));
            let ret = match f.results.len() {
                0 => quote::quote! { () },
                1 => results.next().unwrap(),
                _ => quote::quote! { (#(#results),*) },
            };
            quote::quote! {
                fn #name(&self, #(_: #params),*) -> #ret {
                    loop {}
                }
            }
        });

        let fnname = quote::format_ident!("{}", iface.name.to_snake_case());
        let the_trait = quote::format_ident!("{}", iface.name.to_camel_case());
        quote::quote! {
            fn #fnname() -> &'static impl #fnname::#the_trait {
                struct A;
                impl #fnname::#the_trait for A {
                    #(#methods)*
                }
                &A
            }
        }
    }

    fn quote_ty(iface: &witx2::Interface, ty: &witx2::Type) -> proc_macro2::TokenStream {
        use witx2::Type;
        match *ty {
            Type::U8 => quote::quote! { u8 },
            Type::S8 => quote::quote! { i8 },
            Type::U16 => quote::quote! { u16 },
            Type::S16 => quote::quote! { i16 },
            Type::U32 => quote::quote! { u32 },
            Type::S32 => quote::quote! { i32 },
            Type::U64 => quote::quote! { u64 },
            Type::S64 => quote::quote! { i64 },
            Type::CChar => quote::quote! { u8 },
            Type::Usize => quote::quote! { usize },
            Type::F32 => quote::quote! { f32 },
            Type::F64 => quote::quote! { f64 },
            Type::Char => quote::quote! { char },
            Type::Handle(resource) => {
                let name =
                    quote::format_ident!("{}", iface.resources[resource].name.to_camel_case());
                quote::quote! { &#name }
            }
            Type::Id(id) => quote_id(iface, id),
        }
    }

    fn quote_id(iface: &witx2::Interface, ty: witx2::TypeId) -> proc_macro2::TokenStream {
        use witx2::{Type, TypeDefKind};
        let ty = &iface.types[ty];
        if let Some(name) = &ty.name {
            let name = quote::format_ident!("{}", name.to_camel_case());
            let module = quote::format_ident!("{}", iface.name.to_snake_case());
            return quote::quote! { #module::#name };
        }
        match &ty.kind {
            TypeDefKind::Type(t) => quote_ty(iface, t),
            TypeDefKind::Pointer(t) => {
                let t = quote_ty(iface, t);
                quote::quote! { *mut #t }
            }
            TypeDefKind::ConstPointer(t) => {
                let t = quote_ty(iface, t);
                quote::quote! { *const #t }
            }
            TypeDefKind::List(t) => {
                if *t == Type::Char {
                    quote::quote! { String }
                } else {
                    let t = quote_ty(iface, t);
                    quote::quote! { Vec<#t> }
                }
            }
            TypeDefKind::PushBuffer(_) => panic!("unimplemented push-buffer"),
            TypeDefKind::PullBuffer(_) => panic!("unimplemented pull-buffer"),
            TypeDefKind::Record(r) => {
                let fields = r.fields.iter().map(|f| quote_ty(iface, &f.ty));
                quote::quote! { (#(#fields,)*) }
            }
            TypeDefKind::Variant(v) => {
                if v.is_bool() {
                    quote::quote! { bool }
                } else if let Some(ty) = v.as_option() {
                    let ty = quote_ty(iface, ty);
                    quote::quote! { Option<#ty> }
                } else if let Some((ok, err)) = v.as_expected() {
                    let ok = match ok {
                        Some(ok) => quote_ty(iface, ok),
                        None => quote::quote! { () },
                    };
                    let err = match err {
                        Some(err) => quote_ty(iface, err),
                        None => quote::quote! { () },
                    };
                    quote::quote! { Result<#ok, #err> }
                } else {
                    panic!("unknown variant");
                }
            }
        }
    }
}

#[proc_macro]
#[cfg(feature = "witx-bindgen-gen-wasmtime")]
pub fn wasmtime_import(input: TokenStream) -> TokenStream {
    let tests = generate_tests(input, "import", |_path| {
        (witx_bindgen_gen_wasmtime::Opts::default().build(), true)
    });
    include_rust_sources(tests.iter().map(|p| p.1.join("bindings.rs"))).into()
}

#[proc_macro]
#[cfg(feature = "witx-bindgen-gen-wasmtime")]
pub fn wasmtime_export(input: TokenStream) -> TokenStream {
    let tests = generate_tests(input, "export", |_path| {
        (witx_bindgen_gen_wasmtime::Opts::default().build(), false)
    });
    include_rust_sources(tests.iter().map(|p| p.1.join("bindings.rs"))).into()
}

#[proc_macro]
#[cfg(feature = "witx-bindgen-gen-js")]
pub fn js_import(input: TokenStream) -> TokenStream {
    let tests = generate_tests(input, "import", |_path| {
        (witx_bindgen_gen_js::Opts::default().build(), true)
    });
    let tests = tests.iter().map(|(_iface, test, witx)| {
        let test = test.display().to_string();
        let witx = witx.display().to_string();
        quote::quote! {
            {
                const _: &str = include_str!(#witx);
                #test
            }
        }
    });
    (quote::quote! {
        pub const TESTS: &[&str] = &[#(#tests),*];
    })
    .into()
}

#[proc_macro]
#[cfg(feature = "witx-bindgen-gen-js")]
pub fn js_export(input: TokenStream) -> TokenStream {
    let tests = generate_tests(input, "import", |_path| {
        (witx_bindgen_gen_js::Opts::default().build(), false)
    });
    let tests = tests.iter().map(|(_iface, test, witx)| {
        let test = test.display().to_string();
        let witx = witx.display().to_string();
        quote::quote! {
            {
                const _: &str = include_str!(#witx);
                #test
            }
        }
    });
    (quote::quote! {
        pub const TESTS: &[&str] = &[#(#tests),*];
    })
    .into()
}

fn generate_tests<G>(
    input: TokenStream,
    dir: &str,
    mkgen: impl Fn(&Path) -> (G, bool),
) -> Vec<(witx2::Interface, PathBuf, PathBuf)>
where
    G: Generator,
{
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            eprintln!("panic: {:?}", backtrace::Backtrace::new());
            prev(info);
        }));
    });

    let mut builder = GitignoreBuilder::new("tests");
    for token in input {
        let lit = match token {
            TokenTree::Literal(l) => l.to_string(),
            _ => panic!("invalid input"),
        };
        assert!(lit.starts_with("\""));
        assert!(lit.ends_with("\""));
        builder.add_line(None, &lit[1..lit.len() - 1]).unwrap();
    }
    let ignore = builder.build().unwrap();
    let tests = ignore::Walk::new("tests").filter_map(|d| {
        let d = d.unwrap();
        let path = d.path();
        match ignore.matched(path, d.file_type().map(|d| d.is_dir()).unwrap_or(false)) {
            ignore::Match::None => None,
            ignore::Match::Ignore(_) => Some(d.into_path()),
            ignore::Match::Whitelist(_) => None,
        }
    });
    let mut out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));
    out_dir.push(dir);
    let mut sources = Vec::new();
    let cwd = env::current_dir().unwrap();
    for test in tests {
        let (mut gen, import) = mkgen(&test);
        let mut files = Default::default();
        let iface = witx2::Interface::parse_file(&test).unwrap();
        gen.generate(&iface, import, &mut files);

        let dst = out_dir.join(test.file_stem().unwrap());
        drop(fs::remove_dir_all(&dst));
        fs::create_dir_all(&dst).unwrap();
        for (file, contents) in files.iter() {
            fs::write(dst.join(file), contents).unwrap();
        }
        sources.push((iface, dst, cwd.join(test)));
    }
    sources
}

#[allow(dead_code)]
fn include_rust_sources(srcs: impl Iterator<Item = PathBuf>) -> proc_macro2::TokenStream {
    let mut ret = proc_macro2::TokenStream::new();

    let mut rustfmt = std::process::Command::new("rustfmt");
    for src in srcs {
        rustfmt.arg(&src);
        ret.extend(
            format!("include!(\"{}\");", src.display())
                .parse::<proc_macro2::TokenStream>()
                .unwrap(),
        );
    }
    drop(rustfmt.output());

    return ret;
}
