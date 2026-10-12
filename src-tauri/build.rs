// Reads the version from package.json (the single place it is set, A11), checks that
// Cargo.toml mirrors it, and exposes it to the core as AIONE_VERSION.
fn main() {
    println!("cargo:rerun-if-changed=../package.json");
    let pkg = std::fs::read_to_string("../package.json").expect("read ../package.json");
    let json: serde_json::Value = serde_json::from_str(&pkg).expect("parse ../package.json");
    let version = json["version"]
        .as_str()
        .expect("package.json has a version")
        .to_string();
    let cargo_version = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION");
    if version != cargo_version {
        panic!(
            "Version mismatch: package.json has {version} but src-tauri/Cargo.toml has {cargo_version}. \
             Set the same version in both (package.json is the source)."
        );
    }
    println!("cargo:rustc-env=AIONE_VERSION={version}");
    tauri_build::build()
}
