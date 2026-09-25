use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let linker_path = manifest_dir.join("src/linker.ld");

    println!("cargo:rustc-link-arg=-T{}", linker_path.display());
    println!("cargo:rerun-if-changed={}", linker_path.display());
}
