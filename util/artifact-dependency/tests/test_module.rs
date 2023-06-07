use artifact_dependency::{ArtifactDependencyBuilder, CrateType};

#[test]
fn test() {
    let dep = ArtifactDependencyBuilder::default()
        .build_missing(true)
        .artifact_type(CrateType::CDynamicLibrary)
        .crate_name("confuse_module")
        .build()
        .expect("Couldn't build dependency")
        .search()
        .expect("Couldn't find dependency");

    assert!(dep.exists(), "Dep did not exist");
}
