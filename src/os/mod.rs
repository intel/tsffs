#![allow(unused)]

use std::{collections::HashMap, path::PathBuf};

use crate::source_cov::lcov::Records;

pub mod windows;

#[derive(Debug)]
pub struct DebugInfoConfig<'a> {
    pub system: bool,
    pub user_debug_info: &'a HashMap<String, Vec<PathBuf>>,
    pub coverage: &'a mut Records,
}
