use artifact_dependency::{ArtifactDependencyBuilder, CrateType};

#[test]
fn test() {
    let dep = ArtifactDependencyBuilder::default()
        .build_missing(true)
        .build_always(true)
        .artifact_type(CrateType::CDynamicLibrary)
        .target_name("test_module")
        .crate_name("confuse_module")
        .feature("6.0.166")
        .build()
        .expect("Couldn't build dependency")
        .build()
        .expect("Couldn't find dependency");

    assert!(dep.path.exists(), "Dep did not exist");
}
