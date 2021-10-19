use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

mod imports {
    test_helpers::codegen_c_import!(
        // ...
        "*.witx"

        // TODO: implement async support
        "!async_functions.witx"
    );
}

mod exports {
    test_helpers::codegen_c_export!(
        "*.witx"

        // TODO: implement async support
        "!async_functions.witx"

        // These use preview1 ABI things which are only supported for imports
        "!wasi_snapshot_preview1.witx"

        // TODO: these use push/pull buffer in exports which isn't implemented
        // yet
        "!wasi_next.witx"
        "!host.witx"

        // These use the preview1 ABI which isn't implemented for C exports.
        "!wasi_snapshot_preview1.witx"
        "!typenames.witx"
        "!legacy.witx"
    );
}

fn verify(dir: &str, name: &str) {
    let dir = Path::new(dir);
    let path = PathBuf::from(env::var_os("WASI_SDK_PATH").unwrap());
    let mut cmd = Command::new(path.join("bin/clang"));
    cmd.arg("--sysroot").arg(path.join("share/wasi-sysroot"));
    cmd.arg(dir.join(format!("{}.c", name)));
    cmd.arg("-I").arg(dir);
    cmd.arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-Wno-unused-parameter");
    cmd.arg("-c");
    cmd.arg("-o").arg(dir.join("obj.o"));

    println!("{:?}", cmd);
    let output = match cmd.output() {
        Ok(output) => output,
        Err(e) => panic!("failed to spawn compiler: {}", e),
    };

    if output.status.success() {
        return;
    }
    println!("status: {}", output.status);
    println!("stdout: ------------------------------------------");
    println!("{}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: ------------------------------------------");
    println!("{}", String::from_utf8_lossy(&output.stderr));
    panic!("failed to compile");
}
