// The original links the SDL2 compatibility library from Homebrew. The Rust build links the same library.
fn main() {
    println!("cargo:rustc-link-search=native=/opt/homebrew/opt/sdl2-compat/lib");
    println!("cargo:rustc-link-lib=dylib=SDL2");
    println!("cargo:rerun-if-changed=build.rs");
}
