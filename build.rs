// Build script for qwen3_tts.
//
// When the `mlx` feature is enabled (macOS only), this script:
// 1. Builds a patched copy of the mlx-c submodule via CMake
// 2. Caps MLX Metal kernel source compilation to the highest language revision
//    supported by the current Xcode 17 toolchain
// 3. Emits linker directives for mlx-c, MLX, Metal, and system frameworks

use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    #[cfg(feature = "mlx")]
    build_mlx();
}

/// Build the Apple MLX backend from a patched copy of the upstream `mlx-c`
/// submodule so the fix stays reproducible in Cargo builds.
#[cfg(feature = "mlx")]
fn build_mlx() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    if target_os != "macos" {
        panic!("The `mlx` feature is only supported on macOS. Current target OS: {target_os}");
    }
    if target_arch != "aarch64" {
        eprintln!(
            "Warning: MLX is optimized for Apple Silicon (aarch64). \
             Current target arch: {target_arch}. Metal GPU acceleration may not be available."
        );
    }

    let mlx_c_dir = std::path::PathBuf::from("mlx-c");
    if !mlx_c_dir.join("CMakeLists.txt").exists() {
        panic!(
            "mlx-c submodule not found. Please run:\n\
             \n\
             git submodule update --init --recursive\n\
             \n\
             to clone the mlx-c dependency."
        );
    }

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR must be set"));
    let patched_mlx_c_dir = prepare_patched_mlx_c_source(&mlx_c_dir, &out_dir);

    // Build mlx-c via CMake
    let dst = cmake::Config::new(&patched_mlx_c_dir)
        .define("MLX_BUILD_TESTS", "OFF")
        .define("MLX_BUILD_EXAMPLES", "OFF")
        .define("MLX_BUILD_BENCHMARKS", "OFF")
        .define("BUILD_SHARED_LIBS", "OFF")
        .build();

    // Link paths
    let lib_dir = dst.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // Also check lib64 (some CMake configs use this)
    let lib64_dir = dst.join("lib64");
    if lib64_dir.exists() {
        println!("cargo:rustc-link-search=native={}", lib64_dir.display());
    }

    // Link mlx-c and mlx static libraries
    println!("cargo:rustc-link-lib=static=mlxc");
    println!("cargo:rustc-link-lib=static=mlx");

    // Link macOS system frameworks required by MLX
    println!("cargo:rustc-link-lib=framework=Metal");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=Accelerate");
    println!("cargo:rustc-link-lib=framework=MetalPerformanceShaders");

    // Link C++ standard library
    println!("cargo:rustc-link-lib=c++");

    // Rerun if mlx-c sources change
    println!("cargo:rerun-if-changed=mlx-c/CMakeLists.txt");
    println!("cargo:rerun-if-changed=mlx-c/mlx/c/");
    println!("cargo:rerun-if-changed=build-support/patch_mlx_metal_version.cmake");
}

/// Copy the `mlx-c` submodule into `OUT_DIR` and inject a post-fetch MLX patch
/// so builds stay hermetic and do not mutate the checked-out submodule.
#[cfg(feature = "mlx")]
fn prepare_patched_mlx_c_source(mlx_c_dir: &Path, out_dir: &Path) -> PathBuf {
    let patched_mlx_c_dir = out_dir.join("mlx-c-src");
    if patched_mlx_c_dir.exists() {
        fs::remove_dir_all(&patched_mlx_c_dir).expect("failed to clear patched mlx-c source");
    }

    copy_dir_recursive(mlx_c_dir, &patched_mlx_c_dir);

    let cmake_patch_dir = patched_mlx_c_dir.join("cmake");
    fs::create_dir_all(&cmake_patch_dir).expect("failed to create CMake patch directory");
    fs::copy(
        "build-support/patch_mlx_metal_version.cmake",
        cmake_patch_dir.join("patch_mlx_metal_version.cmake"),
    )
    .expect("failed to copy MLX Metal patch script");

    let cmake_lists_path = patched_mlx_c_dir.join("CMakeLists.txt");
    let cmake_lists =
        fs::read_to_string(&cmake_lists_path).expect("failed to read copied mlx-c CMakeLists.txt");
    let patched_cmake_lists = cmake_lists.replace(
        "  FetchContent_MakeAvailable(mlx)\nendif()",
        "  FetchContent_MakeAvailable(mlx)\n  if(APPLE)\n    execute_process(\n      COMMAND ${CMAKE_COMMAND}\n              -DMLX_SOURCE_DIR=${mlx_SOURCE_DIR}\n              -P ${CMAKE_CURRENT_LIST_DIR}/cmake/patch_mlx_metal_version.cmake\n      COMMAND_ERROR_IS_FATAL ANY)\n  endif()\nendif()",
    );

    if patched_cmake_lists == cmake_lists {
        panic!("failed to inject MLX Metal patch into copied mlx-c CMakeLists.txt");
    }

    fs::write(&cmake_lists_path, patched_cmake_lists)
        .expect("failed to write patched mlx-c CMakeLists.txt");

    patched_mlx_c_dir
}

/// Recursively copy a source directory, skipping nested git metadata so the
/// Cargo build uses a clean self-contained staging tree.
#[cfg(feature = "mlx")]
fn copy_dir_recursive(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("failed to create recursive copy destination");

    for entry in fs::read_dir(source).expect("failed to read recursive copy source") {
        let entry = entry.expect("failed to read directory entry");
        let path = entry.path();
        let target_path = destination.join(entry.file_name());

        if entry.file_name() == ".git" {
            continue;
        }

        let metadata = entry.metadata().expect("failed to read entry metadata");
        if metadata.is_dir() {
            copy_dir_recursive(&path, &target_path);
        } else if metadata.is_file() {
            fs::copy(&path, &target_path).unwrap_or_else(|error| {
                panic!(
                    "failed to copy '{}' to '{}': {error}",
                    path.display(),
                    target_path.display()
                )
            });
        }
    }
}
