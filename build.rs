use vergen::{vergen, Config};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let mut config = Config::default();
    *config.git_mut().commit_timestamp_kind_mut() = vergen::TimestampKind::DateOnly;
    vergen(config).expect("failed to extract build information");
}
