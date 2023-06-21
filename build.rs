use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let make_flags = env::var("CARGO_MAKEFLAGS").expect("Missing CARGO_MAKEFLAGS");

    let out_dir = env::var("OUT_DIR").expect("Missing OUT_DIR");
    let orc_build_dir = Path::new(&out_dir).join("orc");
    let orc_build_dir = orc_build_dir.as_path();

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Missing CARGO_MANIFEST_DIR");
    let orc_src_dir = Path::new(&manifest_dir).join("orc");
    let orc_src_dir = orc_src_dir.as_path();

    if !orc_build_dir.exists() {
        fs::create_dir(orc_build_dir)
            .expect(&format!("Failed to create {}", orc_build_dir.display()));
    }

    run_cmake(orc_src_dir, orc_build_dir);
    run_make(orc_src_dir, orc_build_dir, &make_flags);
    build_bridge(orc_src_dir, orc_build_dir);
    link_bridge(orc_src_dir, orc_build_dir);
    link_cpp_deps(orc_build_dir, orc_build_dir);
}

/// Configures Apache ORC build
fn run_cmake(orc_src_dir: &Path, orc_build_dir: &Path) {
    let status = process::Command::new("cmake")
        .arg(orc_src_dir)
        .arg("-DBUILD_JAVA=OFF")
        .current_dir(orc_build_dir)
        .status()
        .expect("failed to run cmake");

    if status.code().expect("cmake returned no status code") != 0 {
        panic!("cmake returned {}", status);
    }
}

/// Builds Apache ORC C++
fn run_make(_orc_src_dir: &Path, orc_build_dir: &Path, make_flags: &str) {
    // Run make
    let status = process::Command::new("make")
        .env("MAKEFLAGS", make_flags)
        .current_dir(orc_build_dir)
        .status()
        .expect("failed to run make");

    if status.code().expect("make returned no status code") != 0 {
        panic!("make returned {}", status);
    }
}

/// Compiles the C++ <-> Rust bridge code
fn build_bridge(orc_src_dir: &Path, orc_build_dir: &Path) {
    let src_include_path = orc_src_dir.join("c++/include/");
    let src_include_path = src_include_path.to_str().expect(&format!(
        "Could not convert {} to &str",
        src_include_path.display()
    ));
    let build_include_path = orc_build_dir.join("c++/include/");
    let build_include_path = build_include_path.to_str().expect(&format!(
        "Could not convert {} to &str",
        build_include_path.display()
    ));

    cxx_build::bridge("src/lib.rs")
        .include("src")
        .include(src_include_path)
        .include(build_include_path)
        .compile("orcxx");
}

/// Tells rustc where to find the bridge
fn link_bridge(_orc_src_dir: &Path, orc_build_dir: &Path) {
    let liborc_path = orc_build_dir.join("c++/src");
    let liborc_path = liborc_path.to_str().expect(&format!(
        "Could not convert {} to &str",
        liborc_path.display()
    ));
    println!("cargo:rustc-link-search={}", liborc_path);
    println!("cargo:rustc-link-lib=orc");
}

/// Tells rustc to link dependencies of the C++ code
fn link_cpp_deps(_orc_src_dir: &Path, orc_build_dir: &Path) {
    // FIXME: There should be a way to dynamically find the list of libraries to link to...
    for (thirdparty_name, thirdparty_libname) in vec![
        ("lz4", "lz4"),
        ("protobuf", "protobuf"),
        ("snappy", "snappy"),
        ("zlib", "z"),
        ("zstd", "zstd"),
    ] {
        let thirdparty_path = orc_build_dir.join(&format!(
            "c++/libs/thirdparty/{}_ep-install/lib",
            thirdparty_name
        ));
        let thirdparty_path = thirdparty_path.to_str().expect(&format!(
            "Could not convert {} to &str",
            thirdparty_path.display()
        ));
        println!("cargo:rustc-link-search={}", thirdparty_path);
        println!("cargo:rustc-link-lib={}", thirdparty_libname);
    }
}
