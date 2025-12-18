use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "crystal-client-bin")]
pub(crate) struct Cli {
    #[arg(long)]
    pub(crate) config: Option<String>,
    #[arg(long)]
    pub(crate) server: Option<String>,
    #[arg(long)]
    pub(crate) account: Option<String>,
    #[arg(long)]
    pub(crate) password: Option<String>,
    #[arg(long)]
    pub(crate) start: Option<i32>,

    #[arg(long)]
    pub(crate) login_ui: bool,
    #[arg(long)]
    pub(crate) data_dir: Option<String>,

    #[arg(long)]
    pub(crate) lib_test: bool,
    #[arg(long)]
    pub(crate) lib_path: Option<String>,
    #[arg(long, default_value_t = 1084)]
    pub(crate) lib_index: usize,
}
