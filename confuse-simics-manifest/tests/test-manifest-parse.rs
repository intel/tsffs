use std::path::PathBuf;

use confuse_simics_manifest::{package_infos, simics_base_latest, PublicPackageNumber};

const CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[test]
fn test_manifest_latest() {
    let fake_simics_home = PathBuf::from(CARGO_MANIFEST_DIR).join("tests").join("rsrc");
    let latest = simics_base_latest(fake_simics_home).expect("Couldn't get latest SIMICS version");
    assert!(latest.version == *"6.0.157");
}

#[test]
fn test_package_infos() {
    let fake_simics_home = PathBuf::from(CARGO_MANIFEST_DIR).join("tests").join("rsrc");
    let infos = package_infos(fake_simics_home).expect("Couldn't get package infos");

    let base = &infos[&1000];
    let base_latest = &base["6.0.157"];
    assert_eq!(base_latest.build_id, 6191, "Build ID doesn't match!");
}

#[test]
fn test_package_enum_infos() {
    let fake_simics_home = PathBuf::from(CARGO_MANIFEST_DIR).join("tests").join("rsrc");
    let infos = package_infos(fake_simics_home).expect("Couldn't get package infos");

    let base_pkg_num: i64 = PublicPackageNumber::Base.into();
    let base = &infos[&base_pkg_num];
    let base_latest = &base["6.0.157"];
    assert_eq!(base_latest.build_id, 6191, "Build ID doesn't match!");
}
