use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=src/runtime.rs");
    let target = env::var("TARGET").expect("Cargo supplies TARGET");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo supplies OUT_DIR"))
        .join("chad-runtime.bin");
    let status = Command::new(env::var_os("RUSTC").expect("Cargo supplies RUSTC"))
        .args([
            "--crate-name",
            "chad_runtime",
            "--edition=2024",
            "--target",
            &target,
            "-C",
            "opt-level=z",
            "-C",
            "panic=abort",
            "-C",
            "strip=symbols",
            "src/runtime.rs",
            "-o",
        ])
        .arg(output)
        .status()
        .expect("could not start rustc for the Chad runtime");
    assert!(
        status.success(),
        "could not compile the Chad runtime for {target}"
    );
}
