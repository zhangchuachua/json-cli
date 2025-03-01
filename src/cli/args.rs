use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    author,
    version,
    about = "文件处理工具集",
    long_about = "一个用于文件复制和 JSON 处理的命令行工具"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// 要忽略的目录列表，例如: --ignore-dirs node_modules --ignore-dirs .git
    #[arg(long = "ignore-dirs", global = true, value_delimiter = ' ')]
    pub ignore_dirs: Option<Vec<String>>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 复制文件到目标位置
    Copy {
        /// 源文件路径，支持正则，例如: ./(?<lang>[a-z-]{2,}/.*\.json)
        #[arg(short, long)]
        from: PathBuf,

        /// 目标路径，例如: ./other/$lang/
        #[arg(short, long)]
        to: PathBuf,

        /// 是否跳过已存在的文件
        #[arg(long, default_value_t = false)]
        skip_exist: bool,
    },
    /// 修改 JSON 文件内容
    ModifyJson {
        /// 源文件路径
        #[arg(short, long)]
        from: PathBuf,

        /// 目标路径
        #[arg(short, long)]
        to: PathBuf,

        /// JSON 路径表达式，例如: $.meta.*
        #[arg(short = 'p', long)]
        json_path: String,

        /// 是否跳过已存在的文件
        #[arg(long, default_value_t = false)]
        skip_exist: bool,
    },
}
