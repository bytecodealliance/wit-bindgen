use std::path::Path;
use std::process::Command;

mod exports {
    test_helpers::codegen_js_export!(
        // ...
        "*.witx"

        // These use preview1 ABI things which aren't implemented
        "!wasi_snapshot_preview1.witx"
    );
}

mod imports {
    test_helpers::codegen_js_import!(
        "*.witx"

        // This uses buffers, which we don't support in imports just yet
        // TODO: should support this
        "!wasi_next.witx"
        "!host.witx"

        // These use the preview1 ABI which isn't implemented for JS imports.
        "!wasi_snapshot_preview1.witx"
        "!typenames.witx"
        "!legacy.witx"
    );
}

fn verify(dir: &str, name: &str) {
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
        .arg(Path::new(dir).join(&format!("{}.js", name)))
        .status()
        .unwrap();
    assert!(status.success());
}
