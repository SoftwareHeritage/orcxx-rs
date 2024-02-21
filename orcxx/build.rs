use std::env;
use std::fs;
use std::path::Path;
use std::process;

extern crate thiserror;

use thiserror::Error;

const BRIDGE_MODULES: [&str; 5] = [
    "src/kind.rs",
    "src/int128.rs",
    "src/reader.rs",
    "src/memorypool.rs",
    "src/vector.rs",
];

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Could not run CMake: {0}")]
    CmakeStartError(std::io::Error),
    #[error("CMake returned exit code {0}")]
    CmakeStatus(process::ExitStatus),
    #[error("Could not run Make: {0}")]
    MakeStartError(std::io::Error),
    #[error("Make returned exit code {0}")]
    MakeStatus(process::ExitStatus),
}

fn main() {
    if let Err(e) = main_() {
        eprintln!("Failed to build the ORC C++ Core library: {}", e);
        process::exit(1);
    }
}
fn main_() -> Result<(), BuildError> {
    let make_flags = env::var("CARGO_MAKEFLAGS").expect("Missing CARGO_MAKEFLAGS");

    let out_dir = env::var("OUT_DIR").expect("Missing OUT_DIR");
    let out_dir = Path::new(&out_dir);
    let orc_build_dir = out_dir.join("../orc");
    let orc_build_dir = orc_build_dir.as_path();

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Missing CARGO_MANIFEST_DIR");
    let orc_src_dir = Path::new(&manifest_dir).join("orc");
    let orc_src_dir = orc_src_dir.as_path();

    if !orc_build_dir.exists() {
        fs::create_dir(orc_build_dir)
            .unwrap_or_else(|_| panic!("Failed to create {}", orc_build_dir.display()));
    }

    let orc_src_include_dir = orc_src_dir.join("c++/include/");
    let orc_src_include_dir = orc_src_include_dir.to_str().unwrap_or_else(|| {
        panic!(
            "Could not convert {} to &str",
            orc_src_include_dir.display()
        )
    });
    let orc_build_include_dir = orc_build_dir.join("c++/include/");
    let orc_build_include_dir = orc_build_include_dir.to_str().unwrap_or_else(|| {
        panic!(
            "Could not convert {} to &str",
            orc_build_include_dir.display()
        )
    });

    let build = OrcxxBuild {
        orc_src_dir,
        orc_build_dir,
        orc_src_include_dir,
        orc_build_include_dir,
    };

    build.run_cmake()?;
    build.run_make(&make_flags)?;
    build.build_bridge();
    build.link_bridge();
    build.link_cpp_deps();

    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rerun-if-env-changed=ORC_USE_SYSTEM_LIBRARIES");
    println!("cargo:rerun-if-env-changed=ORC_DISABLE_HDFS");
    println!("cargo:rerun-if-changed={}", orc_src_dir.display());
    for module in BRIDGE_MODULES {
        println!("cargo:rerun-if-changed={}/{}", manifest_dir, module);
    }
    println!("cargo:rerun-if-changed={}/src/cpp-utils.hh", manifest_dir);

    Ok(())
}

struct OrcxxBuild<'a> {
    orc_src_dir: &'a Path,
    orc_build_dir: &'a Path,
    orc_src_include_dir: &'a str,
    orc_build_include_dir: &'a str,
}

impl<'a> OrcxxBuild<'a> {
    /// Configures Apache ORC build
    fn run_cmake(&self) -> Result<(), BuildError> {
        let deps_home = vec![
            "PROTOBUF_HOME",
            "SNAPPY_HOME",
            "ZLIB_HOME",
            "LZ4_HOME",
            "ZSTD_HOME",
        ];
        let mut env: Vec<_> = if std::env::var("DOCS_RS").is_ok()
            || std::env::var("ORC_USE_SYSTEM_LIBRARIES").is_ok()
        {
            // Force use of system libraries instead of downloading them
            deps_home
                .into_iter()
                .map(|var_name| {
                    (
                        var_name,
                        std::env::var(var_name).unwrap_or("/usr".to_owned()),
                    )
                })
                .collect()
        } else {
            deps_home
                .into_iter()
                .flat_map(|var_name| std::env::var(var_name).map(|value| (var_name, value)))
                .collect()
        };
        env.push(("CFLAGS", "-fPIC".to_owned()));
        env.push(("CXXFLAGS", "-fPIC".to_owned()));

        let mut command = process::Command::new("cmake");
        let mut command = command
            .arg(self.orc_src_dir)
            .arg("-DBUILD_JAVA=OFF")
            .arg("-DBUILD_TOOLS=OFF")
            .arg("-DBUILD_CPP_TESTS=OFF");
        // It might be necessary to disable linking against libhdfs on some
        // systems for now...
        if std::env::var("ORC_DISABLE_HDFS").is_ok() {
            command = command.arg("-DBUILD_LIBHDFSPP=OFF");
        }

        let status = command
            .envs(env)
            .current_dir(self.orc_build_dir)
            .status()
            .map_err(BuildError::CmakeStartError)?;

        if status.code().expect("cmake returned no status code") == 0 {
            Ok(())
        } else {
            Err(BuildError::CmakeStatus(status))
        }
    }

    /// Builds Apache ORC C++
    fn run_make(&self, make_flags: &str) -> Result<(), BuildError> {
        // Run make
        let status = process::Command::new("make")
            .env("MAKEFLAGS", make_flags)
            .env("CFLAGS", "-fPIC")
            .env("CXXFLAGS", "-fPIC")
            .current_dir(self.orc_build_dir)
            .status()
            .map_err(BuildError::MakeStartError)?;

        if status.code().expect("make returned no status code") == 0 {
            Ok(())
        } else {
            Err(BuildError::MakeStatus(status))
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
        let liborc_path = liborc_path
            .to_str()
            .unwrap_or_else(|| panic!("Could not convert {} to &str", liborc_path.display()));
        println!("cargo:rustc-link-search={}", liborc_path);
        println!("cargo:rustc-link-lib=orc");
    }

    /// Tells rustc to link dependencies of the C++ code
    fn link_cpp_deps(&self) {
        // FIXME: There should be a way to dynamically find the list of libraries to link to...
        for (thirdparty_name, thirdparty_libname) in &[
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
            let thirdparty_path = thirdparty_path.to_str().unwrap_or_else(|| {
                panic!("Could not convert {} to &str", thirdparty_path.display())
            });
            println!("cargo:rustc-link-search={}", thirdparty_path);
            println!("cargo:rustc-link-lib={}", thirdparty_libname);
        }
    }
}
