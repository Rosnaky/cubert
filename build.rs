
fn main() {

    let libpath = format!("{}/model", env!("CARGO_MANIFEST_DIR"));

    println!("cargo:rustc-link-search=native={}", libpath);

    println!("cargo:rustc-link-lib=cudastats");

    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libpath);

    println!("cargo:rerun-if-changed=model/libcudastats.so");
}
