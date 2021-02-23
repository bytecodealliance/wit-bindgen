use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{
    atomic::{AtomicUsize, Ordering::SeqCst},
    Once,
};
use witx_bindgen_core::{witx, Generator};
use witx_bindgen_rust_wasm::RustWasm;

#[test]
fn smoke() {
    witx("");
    witx("(module $x)");
    witx("(module $x (export \"y\" (func)))");
}

#[test]
fn integers() {
    witx("(module $x (export \"y\" (func (param $a u8))))");
    witx("(module $x (export \"y\" (func (param $a s8))))");
    witx("(module $x (export \"y\" (func (param $a u16))))");
    witx("(module $x (export \"y\" (func (param $a s16))))");
    witx("(module $x (export \"y\" (func (param $a u32))))");
    witx("(module $x (export \"y\" (func (param $a s32))))");
    witx("(module $x (export \"y\" (func (param $a u64))))");
    witx("(module $x (export \"y\" (func (param $a s64))))");
}

#[test]
fn floats() {
    witx("(module $x (export \"y\" (func (param $a f32))))");
    witx("(module $x (export \"y\" (func (param $a f64))))");
}

#[test]
fn chars() {
    witx("(module $x (export \"y\" (func (param $a char))))");
    witx("(module $x (export \"y\" (func (result $a char))))");
}

#[test]
fn records() {
    witx("(module $x (export \"y\" (func (param $a (tuple char u32)))))");
    witx("(module $x (export \"y\" (func (result $a (tuple char u32)))))");
    witx("(module $x (export \"y\" (func (result $a char) (result $b u32))))");
    witx(
        "
            (typename $a (record))
            (module $x
                (export \"a\" (func (param $a $a) (result $b $a)))
            )
        ",
    );
    witx(
        "
            (typename $a (record (field $a u32) (field $b f32)))
            (module $x
                (export \"a\" (func (param $a $a) (result $b $a)))
            )
        ",
    );
    witx(
        "
            (typename $a (record (field $a u32) (field $b f32)))
            (typename $b (record (field $a $a)))
            (module $x
                (export \"a\" (func (param $a $b) (result $b $b)))
            )
        ",
    );
}

#[test]
fn variants() {
    witx("(module $x (export \"y\" (func (param $a bool) (result $b bool))))");
    witx("(module $x (export \"y\" (func (param $a (expected (error))) (result $b (expected (error))))))");
    witx(
        "
            (typename $a (enum $a $b $c))
            (module $x
                (export \"a\" (func (param $a $a) (result $b $a)))
            )
        ",
    );
    witx(
        "
            (typename $a (union f32 f64))
            (module $x
                (export \"a\" (func (param $a $a) (result $b $a)))
            )
        ",
    );
    witx(
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
}

#[test]
fn lists() {
    witx("(module $x (export \"y\" (func (param $a (list u8)) (result $b (list u8)))))");
    witx("(module $x (export \"y\" (func (param $a (list char)) (result $b (list char)))))");
    witx("(module $x (export \"y\" (func (param $a (list bool)))))");
    witx("(module $x (export \"y\" (func (result $a (list bool)))))");
    witx("(module $x (export \"y\" (func (param $a (list (list bool))))))");
    witx("(module $x (export \"y\" (func (result $a (list (list bool))))))");
}

static INIT: Once = Once::new();
static CNT: AtomicUsize = AtomicUsize::new(0);

fn witx(src: &str) {
    let base = init();
    let me = CNT.fetch_add(1, SeqCst);
    let doc = witx::parse(src).unwrap();
    let files = RustWasm::new(true, false).generate(&doc, true);
    let dir = base.join(format!("t{}", me));
    std::fs::create_dir(&dir).unwrap();
    for (file, contents) in files.iter() {
        let file = dir.join(file);
        std::fs::write(&file, &contents).unwrap();
        let status = Command::new("rustc")
            .arg("--target=wasm32-wasi")
            .arg(&file)
            .arg("--crate-type=lib")
            .arg("--out-dir")
            .arg(&dir)
            .arg("-Dwarnings")
            .status()
            .unwrap();
        assert!(status.success());
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
