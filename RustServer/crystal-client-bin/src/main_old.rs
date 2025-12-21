use clap::Parser;
use std::path::PathBuf;

use crate::app_config::{LibTestConfig, LoginUiConfig, RuntimeConfig};
use crate::cli::Cli;
use crate::modes::lib_test::run_lib_test;
use crystal_client_config::ClientConfig;

pub fn main() {
    let cli = Cli::parse();

    if cli.login_ui {
        let cfg_path: PathBuf = cli
            .config
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(ClientConfig::default_path);
        let cfg = ClientConfig::load_or_default(&cfg_path).unwrap_or_default();
        let server_addr = cli
            .server
            .clone()
            .unwrap_or_else(|| cfg.network.server_addr.clone());
        let data_dir = cli
            .data_dir
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("../Build/Client/Debug/Data"));
        crate::modes::login_ui::run_login_ui(LoginUiConfig {
            data_dir,
            server_addr,
        });
        return;
    }

    if cli.lib_test {
        let lib_path = cli
            .lib_path
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("../Build/Client/Debug/Data/Prguse.Lib"));
        run_lib_test(LibTestConfig {
            lib_path,
            lib_index: cli.lib_index,
        });
        return;
    }

    let cfg_path: PathBuf = cli
        .config
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(ClientConfig::default_path);

    let cfg = ClientConfig::load_or_default(&cfg_path).unwrap_or_default();
    let server_addr = cli
        .server
        .clone()
        .unwrap_or_else(|| cfg.network.server_addr.clone());
    let account = cli.account.clone().or(cfg.account.clone());
    let start = cli.start.or(cfg.character_index);

    let runtime = RuntimeConfig {
        server_addr,
        account,
        password: cli.password.clone(),
        start,
        config_path: cfg_path.to_string_lossy().to_string(),
    };

    crate::modes::runtime_chat::run_runtime_chat(runtime);
}
