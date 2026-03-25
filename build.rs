use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let gem_dir = manifest_dir.join("dependencies/gem-blockset");

    let dst = cmake::Config::new(&gem_dir)
        .define("CMAKE_CUDA_ARCHITECTURES", "75")
        .define("CMAKE_CUDA_COMPILER", "/opt/cuda/bin/nvcc")
        .define("GEM_BUILD_FFI", "ON")
        .build_target("gem_blockset")
        .build();

    println!("cargo:rustc-link-search=native={}/build", dst.display());
    println!("cargo:rustc-link-lib=static=gem_blockset");

    println!("cargo:rustc-link-search=native=/opt/cuda/lib64");
    println!("cargo:rustc-link-lib=dylib=cudart");

    println!("cargo:rustc-link-lib=dylib=stdc++");

    // Generate gem-blockset FFI Rust bindings
    let bindings = bindgen::Builder::default()
        .header(gem_dir.join("models/ffi.h").to_str().unwrap())
        .generate()
        .expect("failed to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("gem_bindings.rs"))
        .expect("failed to write bindings");

    println!("cargo:rerun-if-changed=dependencies/gem-blockset/models");
}
