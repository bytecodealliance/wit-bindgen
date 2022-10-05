use std::path::Path;
use std::process::Command;

#[rustfmt::skip]
mod exports {
    test_helpers::codegen_py_export!(
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
    test_helpers::codegen_py_import!(
        "*.wit"
    );
}

fn verify(dir: &str, _name: &str) {
    let output = Command::new("mypy")
        .arg(Path::new(dir).join("bindings.py"))
        .arg("--config-file")
        .arg("mypy.ini")
        .output()
        .expect("failed to run `mypy`; do you have it installed?");
    if output.status.success() {
        return;
    }
    panic!(
        "mypy failed

status: {status}

stdout ---
{stdout}

stderr ---
{stderr}",
        status = output.status,
        stdout = String::from_utf8_lossy(&output.stdout).replace("\n", "\n\t"),
        stderr = String::from_utf8_lossy(&output.stderr).replace("\n", "\n\t"),
    );
}
