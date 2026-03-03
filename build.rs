fn main() {
    // Re-link whenever the linker script changes.
    // Without this, cargo would not detect linker.ld modifications.
    println!("cargo:rerun-if-changed=linker.ld");
}
