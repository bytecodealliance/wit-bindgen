use std::path::Path;
use std::process::Command;

mod exports {
    test_helpers::codegen_js_export!(
        // ...
        "*.wit"
    );
}

mod imports {
    test_helpers::codegen_js_import!(
        "*.wit"

        // This uses buffers, which we don't support in imports just yet
        // TODO: should support this
        "!wasi-next.wit"
        "!host.wit"
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
