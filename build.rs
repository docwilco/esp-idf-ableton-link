use std::env;
use std::path::PathBuf;

fn main() {
    let link_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("ableton-link");
    
    // Build the C wrapper
    cc::Build::new()
        .cpp(true)
        .file("src/link_wrapper.cpp")
        .include(link_dir.join("include"))
        .flag_if_supported("-std=c++11")
        .flag_if_supported("-DLINK_PLATFORM_LINUX=1")
        .warnings(false)
        .compile("link_wrapper");

    println!("cargo:rerun-if-changed=src/link_wrapper.cpp");
    println!("cargo:rerun-if-changed=src/link_wrapper.h");
}
