use derive_builder::Builder;
use log::Level;
use std::path::PathBuf;

#[derive(Builder)]
pub struct Fuzzer {
    #[builder(default = "self.default_project()")]
    project: Project,
    #[builder(setter(into), default)]
    input: PathBuf,
    #[builder(setter(into), default)]
    corpus: PathBuf,
    #[builder(setter(into), default)]
    solutions: PathBuf,
    log_level: Level,
    tui: bool,
    grimoire: bool,
}

impl FuzzerBuilder {
    /// Create a new project if a path to an existing project wasn't specified
    fn default_project(&self) -> Result<PathBuf, String> {}
}
