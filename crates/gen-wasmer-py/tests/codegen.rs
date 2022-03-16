use std::path::Path;
use std::process::Command;

mod exports {
    test_helpers::codegen_wasmer_py_export!(
        "*.wit"

        // TODO: implement async support
        "!async-functions.wit"
    );
}

mod imports {
    test_helpers::codegen_wasmer_py_import!(
        "*.wit"

        // TODO: implement async support
        "!async-functions.wit"

        // This uses buffers, which we don't support in imports just yet
        // TODO: should support this
        "!wasi-next.wit"
        "!host.wit"
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
