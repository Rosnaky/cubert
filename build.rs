
fn main() {
    println!("cargo:rustc-link-search=native={}/model", env!("CARGO_MANIFEST_DIR"));

    println!("cargo:rustc-link-lib=cudastats");

    println!("cargo:rerun-if-changed=model/libcudastats.so");
}
