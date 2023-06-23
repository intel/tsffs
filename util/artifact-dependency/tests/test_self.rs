use artifact_dependency::{ArtifactDependencyBuilder, CrateType};

#[test]
fn test() {
    let dep = ArtifactDependencyBuilder::default()
        .build_missing(true)
        .artifact_type(CrateType::CDynamicLibrary)
        .crate_name("artifact-dependency")
        .build()
        .expect("Couldn't build dependency")
        .build()
        .expect("Couldn't find dependency");

    assert!(dep.path.exists(), "Dep did not exist");
}
