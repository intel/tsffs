use std::path::PathBuf;

use chrono::Datelike;
use confuse_simics_manifest::{simics_latest, PackageNumber, SimicsManifest};
use serde_yaml::from_reader;

const SIMICS_GENERATED_MANIFEST: &str =
    include_str!("./rsrc/manifests/generated-manifest-6b26826cf3358979ee3b.smf");

const CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[test]
fn test_parse_simics_manifest() {
    let manifest: SimicsManifest =
        from_reader(SIMICS_GENERATED_MANIFEST.as_bytes()).expect("Could not deserialize manifest.");
    assert!(manifest.manifest_format == 2);
    assert!(manifest.packages[&PackageNumber::Base].description == Some("Simics Base".to_string()));
    assert!(manifest.packages[&PackageNumber::QuickStartPlatform].version == "6.0.65".to_string());
}

#[test]
fn test_manifest_latest() {
    let fake_simics_home = PathBuf::from(CARGO_MANIFEST_DIR).join("tests").join("rsrc");
    let latest = simics_latest(fake_simics_home).expect("Couldn't get latest SIMICS version");
    assert!(latest.date.year() == 2022);
    assert!(latest.packages[&PackageNumber::Base].version == "6.0.157".to_string());
}
