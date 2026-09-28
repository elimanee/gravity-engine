//! Lets the build find libopenmpt when it is not in a standard location
//! (e.g. on Windows): set `OPENMPT_LIB_DIR` to the folder with its import
//! library, named `openmpt.lib` (MSVC) or `libopenmpt.a` / `libopenmpt.dll.a`.

fn main() {
    println!("cargo:rerun-if-env-changed=OPENMPT_LIB_DIR");
    if let Ok(dir) = std::env::var("OPENMPT_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
    }
}
