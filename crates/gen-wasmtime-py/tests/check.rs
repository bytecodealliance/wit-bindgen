use std::path::Path;
use std::process::Command;

mod imports {
    test_codegen::py_import!(
        "*.witx"

        // The python generator doesn't support the legacy witx features at this
        // time.
        "!legacy.witx"
        "!wasi_snapshot_preview1.witx"
    );
}

mod exports {
    test_codegen::py_export!(
        "*.witx"

        // This uses buffers, which we don't support in exports just yet
        // TODO: should support this
        "!wasi_next.witx"
        "!host.witx"
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
