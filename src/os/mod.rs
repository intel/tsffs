#![allow(unused)]

use std::{collections::HashMap, path::PathBuf};

pub mod windows;

#[derive(Debug, Clone)]
pub struct DebugInfoConfig<'a> {
    pub system: bool,
    pub user_debug_info: &'a HashMap<String, Vec<PathBuf>>,
}
