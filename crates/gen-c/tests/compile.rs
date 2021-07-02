use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

mod imports {
    use super::verify;
    test_codegen::c_import!(
        // ...
        // "*.witx"
        "integers.witx"
        "empty.witx"
        "floats.witx"
        "smoke.witx"
        "records.witx"
        "variants.witx"
        "flags.witx"
    );
}

// mod exports {
//     test_codegen::js_export!(
//         "*.witx"

//         // These use preview1 ABI things which are only supported for imports
//         "!host.witx"
//         "!typenames.witx"
//         "!wasi_snapshot_preview1.witx"

//         // This uses buffers, which we don't support in exports just yet
//         // TODO: should support this
//         "!wasi_next.witx"
//     );
// }

fn verify(dir: &str) {
    let dir = Path::new(dir);
    let path = PathBuf::from(env::var_os("WASI_SDK_PATH").unwrap());
    let mut cmd = Command::new(path.join("bin/clang"));
    cmd.arg("--sysroot").arg(path.join("share/wasi-sysroot"));
    cmd.arg(dir.join("bindings.c"));
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
