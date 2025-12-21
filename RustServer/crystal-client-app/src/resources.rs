//! 全局资源定义

use bevy::prelude::*;
use std::path::PathBuf;

/// 运行时配置
#[derive(Resource, Clone, Debug)]
pub struct RuntimeConfig {
    /// 服务器地址
    pub server_addr: String,
    /// 账号（可选）
    pub account: Option<String>,
    /// 密码（可选）
    pub password: Option<String>,
    /// 角色索引（可选）
    pub start: Option<i32>,
    /// 配置文件路径
    pub config_path: String,
}

/// 登录 UI 配置
#[derive(Resource, Clone, Debug)]
pub struct LoginUiConfig {
    /// 数据目录路径
    pub data_dir: PathBuf,
    /// 服务器地址
    pub server_addr: String,
}

/// Lib 测试配置
#[derive(Resource, Clone, Debug)]
pub struct LibTestConfig {
    /// Lib 文件路径
    pub lib_path: PathBuf,
    /// Lib 索引
    pub lib_index: usize,
}

