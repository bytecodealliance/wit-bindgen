use std::path::Path;
use std::process::Command;

mod imports {
    test_codegen::js_import!(
        // ...
        "*.witx"

        // These use preview1 ABI things which are only supported for imports
        // "!host.witx"
        // "!typenames.witx"
        "!wasi_snapshot_preview1.witx"

        // This uses handles, which we don't support in exports just yet
        // TODO: should support this
        // "!wasi_types.witx"
        // "!wasi_next.witx"

        // If you want to exclude other test you can include it here with
        // gitignore glob syntax:
        //
        // "!wasm.witx"
    );
}

mod exports {
    test_codegen::js_export!(
        "*.witx"

        // These use preview1 ABI things which are only supported for imports
        "!host.witx"
        "!typenames.witx"
        "!wasi_snapshot_preview1.witx"

        // This uses buffers, which we don't support in exports just yet
        // TODO: should support this
        "!wasi_next.witx"
    );
}

fn verify(dir: &str) {
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
