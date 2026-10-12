//! The app version. Set once in package.json; build.rs checks Cargo.toml and exports it.

/// e.g. "4.0.0-alpha.1"
pub const VERSION: &str = env!("AIONE_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn version_matches_package_json() {
        let pkg = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../package.json"))
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&pkg).unwrap();
        assert_eq!(json["version"].as_str().unwrap(), super::VERSION);
        assert_eq!(env!("CARGO_PKG_VERSION"), super::VERSION);
    }

    #[test]
    fn tauri_conf_reads_version_from_package_json() {
        let conf = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tauri.conf.json"))
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&conf).unwrap();
        assert_eq!(json["version"].as_str().unwrap(), "../package.json");
    }
}
