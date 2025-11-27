fn main() {
    // Track our own files
    println!("cargo:rerun-if-changed=src/link_wrapper.cpp");
    println!("cargo:rerun-if-changed=include/link_wrapper.h");
    println!("cargo:rerun-if-changed=CMakeLists.txt");
    println!("cargo:rerun-if-changed=Kconfig");

    // Track all Link headers
    for entry in glob::glob("ableton-link/include/**/*.hpp").unwrap().flatten() {
        println!("cargo:rerun-if-changed={}", entry.display());
    }

    // Track cmake_include files
    for entry in glob::glob("ableton-link/cmake_include/**/*.cmake").unwrap().flatten() {
        println!("cargo:rerun-if-changed={}", entry.display());
    }

    // Track extensions (abl_link C wrapper)
    for pattern in [
        "ableton-link/extensions/**/*.hpp",
        "ableton-link/extensions/**/*.h",
        "ableton-link/extensions/**/*.cpp",
        "ableton-link/extensions/**/*.c",
        "ableton-link/extensions/**/*.cmake",
        "ableton-link/extensions/**/CMakeLists.txt",
    ] {
        for entry in glob::glob(pattern).unwrap().flatten() {
            println!("cargo:rerun-if-changed={}", entry.display());
        }
    }

    // Track Link's own CMakeLists.txt files
    for entry in glob::glob("ableton-link/**/CMakeLists.txt").unwrap().flatten() {
        println!("cargo:rerun-if-changed={}", entry.display());
    }
}
