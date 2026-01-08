fn main() {
    // Linker scripts for defmt and memory layout
    // Note: linkall.x must be last to avoid issues with flip-link
    println!("cargo:rustc-link-arg=-Tdefmt.x");
    println!("cargo:rustc-link-arg=-Tlinkall.x");
}
