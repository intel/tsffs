use std::collections::HashMap;

use serde::Deserialize;
use serde_yaml::from_reader;

const SIMICS_GENERATED_MANIFEST: &str =
    include_str!("./rsrc/generated-manifest-6b26826cf3358979ee3b.smf");

// TODO: Is there a way to have build-specific tests? I don't think so...
// These are shared with build.rs
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct SimicsManifestProfile {
    description: String,
    name: String,
    platform_script: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct SimicsManifestPackage {
    description: Option<String>,
    version: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct SimicsManifest {
    manifest_format: u64,
    name: String,
    group: String,
    version: String,
    date: String,
    description: String,
    profiles: HashMap<String, SimicsManifestProfile>,
    packages: HashMap<u64, SimicsManifestPackage>,
}

#[test]
fn test_parse_simics_manifest() {
    let manifest: SimicsManifest = from_reader(SIMICS_GENERATED_MANIFEST.as_bytes()).unwrap();
    assert!(manifest.manifest_format == 2);
    assert!(manifest.packages[&1000].description == Some("Simics Base".to_string()));
    assert!(manifest.packages[&2096].version == "6.0.65".to_string());
}
