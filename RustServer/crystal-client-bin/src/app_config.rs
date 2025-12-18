use bevy::prelude::*;
use std::path::PathBuf;

#[derive(Resource, Clone)]
pub(crate) struct RuntimeConfig {
    pub(crate) server_addr: String,
    pub(crate) account: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) start: Option<i32>,
    pub(crate) config_path: String,
}

#[derive(Resource, Clone)]
pub(crate) struct LibTestConfig {
    pub(crate) lib_path: PathBuf,
    pub(crate) lib_index: usize,
}

#[derive(Resource, Clone)]
pub(crate) struct LoginUiConfig {
    pub(crate) data_dir: PathBuf,
    pub(crate) server_addr: String,
}
