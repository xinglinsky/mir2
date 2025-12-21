//! 登录场景逻辑

use bevy::prelude::*;
use crystal_client_net::NetClient;
use crystal_shared_proto::login::{CLogin, CClientVersion};

/// 登录场景状态
#[derive(Resource)]
pub struct LoginSceneState {
    /// 账号
    pub account: String,
    /// 密码
    pub password: String,
    /// 网络客户端
    pub net: Option<NetClient>,
    /// 是否已连接
    pub connected: bool,
    /// 是否已发送版本检查
    pub sent_version: bool,
    /// 版本检查是否通过
    pub version_checked: bool,
}

impl Default for LoginSceneState {
    fn default() -> Self {
        Self {
            account: String::new(),
            password: String::new(),
            net: None,
            connected: false,
            sent_version: false,
            version_checked: false,
        }
    }
}

/// 验证账号输入
pub fn validate_account(account: &str) -> bool {
    let len = account.chars().count();
    len >= 3 && len <= 15 && account.chars().all(|c| c.is_ascii_alphanumeric())
}

/// 验证密码输入
pub fn validate_password(password: &str) -> bool {
    let len = password.chars().count();
    len >= 5 && len <= 15 && password.chars().all(|c| c.is_ascii_alphanumeric())
}

/// 触发登录
pub fn trigger_login(
    state: &mut LoginSceneState,
) -> Result<(), String> {
    if !validate_account(&state.account) {
        return Err("账号长度必须在 3-15 个字符之间，且只能包含字母和数字".to_string());
    }
    
    if !validate_password(&state.password) {
        return Err("密码长度必须在 5-15 个字符之间，且只能包含字母和数字".to_string());
    }
    
    // TODO: 发送登录包
    // if let Some(net) = &state.net {
    //     let pkt = CLogin {
    //         account_id: state.account.clone(),
    //         password: state.password.clone(),
    //     };
    //     net.send_raw(pkt.encode()?)?;
    // }
    
    Ok(())
}

