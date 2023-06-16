use derive_builder::Builder;
use log::Level;
use std::path::PathBuf;

// #[derive(Builder)]
// pub struct Fuzzer {
//     project: Project,
//     #[builder(setter(into), default)]
//     input: PathBuf,
//     #[builder(setter(into), default)]
//     corpus: PathBuf,
//     #[builder(setter(into), default)]
//     solutions: PathBuf,
//     log_level: Level,
//     tui: bool,
//     grimoire: bool,
// }
