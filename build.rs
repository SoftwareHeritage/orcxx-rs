use std::env;
use std::fs;
use std::path::Path;
use std::process;

const BRIDGE_MODULES: [&str; 5] = [
    "src/kind.rs",
    "src/int128.rs",
    "src/reader.rs",
    "src/memorypool.rs",
    "src/vector.rs",
];

fn main() {
    let make_flags = env::var("CARGO_MAKEFLAGS").expect("Missing CARGO_MAKEFLAGS");

    let out_dir = env::var("OUT_DIR").expect("Missing OUT_DIR");
    let out_dir = Path::new(&out_dir);
    let orc_build_dir = out_dir.join("orc");
    let orc_build_dir = orc_build_dir.as_path();

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Missing CARGO_MANIFEST_DIR");
    let orc_src_dir = Path::new(&manifest_dir).join("orc");
    let orc_src_dir = orc_src_dir.as_path();

    if !orc_build_dir.exists() {
        fs::create_dir(orc_build_dir)
            .expect(&format!("Failed to create {}", orc_build_dir.display()));
    }

    let orc_src_include_dir = orc_src_dir.join("c++/include/");
    let orc_src_include_dir = orc_src_include_dir.to_str().expect(&format!(
        "Could not convert {} to &str",
        orc_src_include_dir.display()
    ));
    let orc_build_include_dir = orc_build_dir.join("c++/include/");
    let orc_build_include_dir = orc_build_include_dir.to_str().expect(&format!(
        "Could not convert {} to &str",
        orc_build_include_dir.display()
    ));

    let build = OrcxxBuild {
        orc_src_dir,
        orc_build_dir,
        orc_src_include_dir,
        orc_build_include_dir,
    };

    build.run_cmake();
    build.run_make(&make_flags);
    build.build_bridge();
    build.link_bridge();
    build.link_cpp_deps();

    println!("cargo:rerun-if-changed={}", orc_src_dir.display());
    for module in BRIDGE_MODULES {
        println!("cargo:rerun-if-changed={}/{}.rs", manifest_dir, module);
    }
    println!("cargo:rerun-if-changed={}/src/cpp-utils.hh", manifest_dir);
}

struct OrcxxBuild<'a> {
    orc_src_dir: &'a Path,
    orc_build_dir: &'a Path,
    orc_src_include_dir: &'a str,
    orc_build_include_dir: &'a str,
}

impl<'a> OrcxxBuild<'a> {
    /// Configures Apache ORC build
    fn run_cmake(&self) {
        let status = process::Command::new("cmake")
            .arg(self.orc_src_dir)
            .arg("-DBUILD_JAVA=OFF")
            .current_dir(self.orc_build_dir)
            .status()
            .expect("failed to run cmake");

        if status.code().expect("cmake returned no status code") != 0 {
            panic!("cmake returned {}", status);
        }
    }

    /// Builds Apache ORC C++
    fn run_make(&self, make_flags: &str) {
        // Run make
        let status = process::Command::new("make")
            .env("MAKEFLAGS", make_flags)
            .current_dir(self.orc_build_dir)
            .status()
            .expect("failed to run make");

        if status.code().expect("make returned no status code") != 0 {
            panic!("make returned {}", status);
        }
    }

    /// Compiles the C++ <-> Rust bridge code
    fn build_bridge(&self) {
        cxx_build::bridges(BRIDGE_MODULES)
            .include("src")
            .include(self.orc_src_include_dir)
            .include(self.orc_build_include_dir)
            .compile("orcxx");
    }

    /// Tells rustc where to find the bridge
    fn link_bridge(&self) {
        let liborc_path = self.orc_build_dir.join("c++/src");
        let liborc_path = liborc_path.to_str().expect(&format!(
            "Could not convert {} to &str",
            liborc_path.display()
        ));
        println!("cargo:rustc-link-search={}", liborc_path);
        println!("cargo:rustc-link-lib=orc");
    }

    /// Tells rustc to link dependencies of the C++ code
    fn link_cpp_deps(&self) {
        // FIXME: There should be a way to dynamically find the list of libraries to link to...
        for (thirdparty_name, thirdparty_libname) in vec![
            ("lz4", "lz4"),
            ("protobuf", "protobuf"),
            ("snappy", "snappy"),
            ("zlib", "z"),
            ("zstd", "zstd"),
        ] {
            let thirdparty_path = self.orc_build_dir.join(&format!(
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
}
