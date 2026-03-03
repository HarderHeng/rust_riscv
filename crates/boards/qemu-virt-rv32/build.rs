use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Get the linker script path
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let linker_script = PathBuf::from("linker.ld");

    // Tell cargo to rerun this script if the linker script changes
    println!("cargo:rerun-if-changed=linker.ld");

    // Copy the linker script to the output directory
    fs::copy(&linker_script, out_dir.join("linker.ld")).unwrap();

    // Tell the linker where to find the script
    println!("cargo:rustc-link-arg=-T{}/linker.ld", out_dir.display());
}
