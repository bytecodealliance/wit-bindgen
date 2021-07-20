use std::path::Path;
use std::process::Command;

mod imports {
    test_codegen::js_import!(
        // ...
        "*.witx"

        // These use preview1 ABI things which aren't implemented
        "!wasi_snapshot_preview1.witx"
    );
}

mod exports {
    test_codegen::js_export!(
        "*.witx"

        // This uses buffers, which we don't support in exports just yet
        // TODO: should support this
        "!wasi_next.witx"
        "!host.witx"
    );
}

fn verify(dir: &str, _name: &str) {
    let (cmd, args) = if cfg!(windows) {
        ("cmd.exe", &["/c", "npx.cmd"] as &[&str])
    } else {
        ("npx", &[] as &[&str])
    };

    let status = Command::new(cmd)
        .args(args)
        .arg("eslint")
        .arg("-c")
        .arg(".eslintrc.js")
        .arg(Path::new(dir).join("bindings.js"))
        .status()
        .unwrap();
    assert!(status.success());
}
