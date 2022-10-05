use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[rustfmt::skip]
mod imports {
    test_helpers::codegen_c_import!(
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
mod exports {
    test_helpers::codegen_c_export!(
        "*.wit"
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
