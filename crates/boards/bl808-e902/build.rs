use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=linker.ld");
    fs::copy("linker.ld", out_dir.join("linker.ld")).unwrap();
    println!("cargo:rustc-link-arg=-T{}/linker.ld", out_dir.display());
}
