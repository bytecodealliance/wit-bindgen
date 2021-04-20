use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{
    atomic::{AtomicUsize, Ordering::SeqCst},
    Once,
};
use witx_bindgen_gen_core::{witx, Files, Generator};

#[test]
fn smoke() {
    import("");
    export("", "include!(\"bindings.rs\");");
    import("(module $x)");
    export("(module $x)", "include!(\"bindings.rs\");");
    import("(module $x (export \"y\" (func)))");
    export(
        "(module $x (export \"y\" (func)))",
        r#"
            include!("bindings.rs");

            fn y() {}
        "#,
    );
}

#[test]
fn integers() {
    import("(module $x (export \"y\" (func (param $a u8))))");
    import("(module $x (export \"y\" (func (param $a s8))))");
    import("(module $x (export \"y\" (func (param $a u16))))");
    import("(module $x (export \"y\" (func (param $a s16))))");
    import("(module $x (export \"y\" (func (param $a u32))))");
    import("(module $x (export \"y\" (func (param $a s32))))");
    import("(module $x (export \"y\" (func (param $a u64))))");
    import("(module $x (export \"y\" (func (param $a s64))))");

    export(
        "(module $x (export \"k\" (func
            (param $a u8)
            (param $b s8)
            (param $c u16)
            (param $d s16)
            (param $e u32)
            (param $f s32)
            (param $g u64)
            (param $h s64)
            (result $r1 u8)
            (result $r2 u16)
        )))",
        r#"
            include!("bindings.rs");

            fn k(
                _: u8,
                _: i8,
                _: u16,
                _: i16,
                _: u32,
                _: i32,
                _: u64,
                _: i64,
            ) -> (u8, u16) {
                (0, 0)
            }
        "#,
    );
}

#[test]
fn floats() {
    import("(module $x (export \"y\" (func (param $a f32))))");
    import("(module $x (export \"y\" (func (param $a f64))))");

    export(
        "(module $x (export \"k\" (func
            (param $a f32)
            (param $b f64)
            (result $r1 f64)
            (result $r2 f32)
        )))",
        r#"
            include!("bindings.rs");

            fn k(
                a: f32,
                b: f64,
            ) -> (f64, f32) {
                (b, a)
            }
        "#,
    );
}

#[test]
fn chars() {
    import("(module $x (export \"y\" (func (param $a char))))");
    import("(module $x (export \"y\" (func (result $a char))))");
    export(
        "(module $x (export \"y\" (func (result $a char))))",
        r#"
            include!("bindings.rs");

            fn y() -> char {
                'x'
            }
        "#,
    );
    export(
        "(module $x (export \"y\" (func (param $a char))))",
        r#"
            include!("bindings.rs");

            fn y(_: char) {
                // ...
            }
        "#,
    );
}

#[test]
fn records() {
    import("(module $x (export \"y\" (func (param $a (tuple char u32)))))");
    import("(module $x (export \"y\" (func (result $a (tuple char u32)))))");
    import("(module $x (export \"y\" (func (result $a char) (result $b u32))))");
    import(
        "
            (typename $a (record))
            (module $x
                (export \"a\" (func (param $a $a) (result $b $a)))
            )
        ",
    );
    import(
        "
            (typename $a (record (field $a u32) (field $b f32)))
            (module $x
                (export \"a\" (func (param $a $a) (result $b $a)))
            )
        ",
    );
    import(
        "
            (typename $a (record (field $a u32) (field $b f32)))
            (typename $b (record (field $a $a)))
            (module $x
                (export \"a\" (func (param $a $b) (result $b $b)))
            )
        ",
    );
    export(
        "
            (typename $a (record (field $a u32) (field $b f32)))
            (typename $b (record (field $a $a)))
            (module $x
                (export \"y\" (func
                    (param $a (tuple s32 u32))
                    (result $b (tuple f64))
                ))
                (export \"z\" (func
                    (param $a $b)
                    (result $b $a)
                ))
            )
        ",
        r#"
            include!("bindings.rs");

            fn y(_: (i32, u32)) -> (f64,) {
                (0.0,)
            }

            fn z(a: B) -> A {
                a.a
            }
        "#,
    );
}

#[test]
fn variants() {
    import("(module $x (export \"y\" (func (param $a bool) (result $b bool))))");
    import("(module $x (export \"y\" (func (param $a (expected (error))) (result $b (expected (error))))))");
    import(
        "
            (typename $a (enum $a $b $c))
            (module $x
                (export \"a\" (func (param $a $a) (result $b $a)))
            )
        ",
    );
    import(
        "
            (typename $a (union f32 f64))
            (module $x
                (export \"a\" (func (param $a $a) (result $b $a)))
            )
        ",
    );
    import(
        "
            (typename $a (variant
                (case $a s8)
                (case $b f32)
                (case $c)
                (case $d (tuple f64 f64))
            ))
            (module $x
                (export \"a\" (func (param $a $a) (result $b $a)))
            )
        ",
    );
    export(
        "
            (typename $a (union f32 u32))
            (typename $b (variant
                (case $a s8)
                (case $b f32)
                (case $c)
                (case $d (tuple f64 f64))
            ))
            (module $x
                (export \"y\" (func
                    (param $a bool)
                    (result $b $a)
                ))
                (export \"z\" (func
                    (param $a $b)
                    (result $b $b)
                ))
            )
        ",
        r#"
            include!("bindings.rs");

            fn y(_: bool) -> A {
                A::V1(1)
            }

            fn z(_: B) -> B {
                B::C
            }
        "#,
    );
}

#[test]
fn lists() {
    import("(module $x (export \"y\" (func (param $a (list u8)) (result $b (list u8)))))");
    import("(module $x (export \"y\" (func (param $a (list char)) (result $b (list char)))))");
    import("(module $x (export \"y\" (func (param $a (list bool)))))");
    import("(module $x (export \"y\" (func (result $a (list bool)))))");
    import("(module $x (export \"y\" (func (param $a (list (list bool))))))");
    import("(module $x (export \"y\" (func (result $a (list (list bool))))))");

    export(
        "
            (module $x
                (export \"y\" (func
                    (param $a (list char))
                    (param $b (list u8))
                    (result $c (list char))
                    (result $d (list u8))
                    (result $e (list (list u8)))
                ))
            )
        ",
        r#"
            include!("bindings.rs");

            fn y(a: String, b: Vec<u8>) -> (String, Vec<u8>, Vec<Vec<u8>>) {
                (a, b.clone(), vec![b])
            }
        "#,
    );
}

#[test]
fn options() {
    import("(module $x (export \"y\" (func (param $a (option u8)) (result $b (option u8)))))");

    export(
        "
            (module $x
                (export \"y\" (func
                    (param $a (option (list char)))
                    (result $b (option (option char)))
                ))
            )
        ",
        r#"
            include!("bindings.rs");

            fn y(a: Option<String>) -> Option<Option<char>> {
                drop(a);
                Some(Some('x'))
            }
        "#,
    );
}

static INIT: Once = Once::new();
static CNT: AtomicUsize = AtomicUsize::new(0);

fn import(src: &str) {
    witx(src, None)
}

fn export(src: &str, rust: &str) {
    witx(src, Some(rust))
}

fn witx(src: &str, rust: Option<&str>) {
    let base = init();
    let doc = witx::parse(src).unwrap();
    for unchecked in [false, true].iter() {
        let me = CNT.fetch_add(1, SeqCst);
        let mut opts = witx_bindgen_gen_rust_wasm::Opts::default();
        opts.rustfmt = true;
        opts.unchecked = *unchecked;
        let mut files = Files::default();
        opts.build().generate(&doc, rust.is_none(), &mut files);
        let dir = base.join(format!("t{}", me));
        std::fs::create_dir(&dir).unwrap();
        for (file, contents) in files.iter() {
            let file = dir.join(file);
            std::fs::write(&file, &contents).unwrap();
            let mut cmd = Command::new("rustc");
            cmd.arg("--target=wasm32-wasi")
                .arg("--crate-type=lib")
                .arg("--out-dir")
                .arg(&dir)
                .arg("-Dwarnings")
                .arg("-Adead-code");
            match rust {
                Some(contents) => {
                    let rust = dir.join("lib.rs");
                    std::fs::write(&rust, contents).unwrap();
                    cmd.arg(&rust);
                }
                None => {
                    cmd.arg(&file);
                }
            }
            let status = cmd.status().unwrap();
            assert!(status.success());
        }
    }
}

fn init() -> PathBuf {
    let mut me = std::env::current_exe().unwrap();
    me.pop();
    let dst = me.join("tmp");
    INIT.call_once(|| {
        drop(fs::remove_dir_all(&dst));
        fs::create_dir(&dst).unwrap();
    });
    return dst;
}
