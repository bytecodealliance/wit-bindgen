use std::path::Path;
use std::process::Command;

#[rustfmt::skip]
mod exports {
    test_helpers::codegen_js_export!(
        "*.wit"

        // If you want to exclude a specific test you can include it here with
        // gitignore glob syntax:
        //
        // "!wasm.wit"
        // "!host.wit"
        //
        //
        // Similarly you can also just remove the `*.wit` glob and list tests
        // individually if you're debugging.
    );
}

#[rustfmt::skip]
mod imports {
    test_helpers::codegen_js_import!(
        "*.wit"
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
