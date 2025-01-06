use std::{
    fs::{self},
    path::Path,
};

use clap::{error::ErrorKind, ColorChoice, CommandFactory, Parser, Subcommand};
use jh::util::{normalize_path, path_to_normalized_str, replace_with};
use log::{error, info};
use serde_json::Value;

// TODO 需要调整报错的逻辑，需要整体性
#[derive(Parser)]
#[command(name = "JSON Helper", version, about, long_about = None, color = ColorChoice::Always)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Copy files
    Cp {
        /// source file path
        source: String,
        /// target path
        target: Vec<String>,
        #[arg(short, long)]
        base_path: Option<String>,
        /// Quiet mode
        #[arg(short, long, default_value_t = false)]
        quiet: bool,
    },

    /// 从 json-path 中提取数据并复制到目标文件中
    // 例如使用 source $.meta.* target 希望把 source 的 meta 值覆盖到 target 中去；如果 target 没有 meta 那么就新建 meta；需要保证尽量保证 meta 在 target 中的位置与 source 一致；
    CpJsonPath {
        /// source file path
        source: String,

        /// json path 语法源自： https://github.com/besok/jsonpath-rust
        json_path: String,

        /// target path
        target: Vec<String>,

        #[arg(short, long)]
        base_path: Option<String>,
    },
}

fn main() {
    // 在代码中设置 log 等级，kt 是当前的包名，意味开启所有级别的日志打印；
    std::env::set_var("RUST_LOG", "jh");
    env_logger::init();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Cp {
            source,
            target,
            base_path,
            quiet,
        } => {
            cp(source, target, base_path, quiet);
        }

        Commands::CpJsonPath {
            source,
            json_path,
            target,
            base_path,
        } => {
            match cp_json_path(source, json_path, target, base_path) {
                Ok(_) => {
                    info!("Command cp-json-path success");
                }
                Err(e) => {
                    error!("Command cp-json-path failed -- {}", e);
                }
            };
        }
    }
}

fn cp(source: &str, target: &Vec<String>, base_path: &Option<String>, quiet: &bool) {
    let base_path = base_path.clone().unwrap_or_default();

    // rust 不需要处理路径中的 ./ ../ 也可以实现其功能，比如要从 /Users/name/Desktop/./kt/main.rs  复制文件，也可以成功；
    // 如果要像将其格式化为常规的样式，比如 /Users/name/Desktop/kt/main.rs 那么可以使用 canonicalize; 但是这个函数在遇到两种情况下会失败
    // 1. 当前目录不存在
    // 2. 当前目录不是文件夹
    // 对于目前这个项目来说，不能使用它，那么就只能自己进行格式化；
    let source_path = Path::new(&base_path).join(source);

    {
        let mut cmd = Cli::command();
        if !source_path.exists() {
            cmd.error(
                ErrorKind::InvalidValue,
                format!(
                    "source path not exists, source path: {}",
                    source_path.display()
                ),
            )
            .exit();
        }
        if !source_path.is_file() {
            cmd.error(
                ErrorKind::InvalidValue,
                format!(
                    "source path is not a file, source path: {}",
                    source_path.display()
                ),
            )
            .exit();
        }
    }

    for target_path in target {
        let mut target_path = Path::new(&base_path).join(target_path);

        if target_path.is_dir() {
            target_path = target_path.join(source_path.file_name().unwrap());
        }

        match fs::copy(&source_path, &target_path) {
            Ok(_) => {
                if !quiet {
                    println!(
                        "copy success: {} -> {}",
                        normalize_path(&source_path).display(),
                        normalize_path(&target_path).display()
                    );
                }
            }
            Err(e) => {
                println!("copy failed: {}", e);
            }
        };
    }
}

fn cp_json_path(
    source: &str,
    json_path: &str,
    target_json_path: &Vec<String>,
    base_path: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_path = base_path.clone().unwrap_or_default();
    let source_path = Path::new(&base_path).join(source);

    {
        let mut cmd = Cli::command();
        let tmp_path = path_to_normalized_str(&source_path).unwrap();
        if !source_path.exists() {
            cmd.error(
                ErrorKind::InvalidValue,
                format!("source path not exists, source path: {}", tmp_path),
            )
            .exit();
        }
        if !source_path.is_file() {
            cmd.error(
                ErrorKind::InvalidValue,
                format!("source path is not a file, source path: {}", tmp_path),
            )
            .exit();
        }
        if source_path.extension().unwrap().ne("json") {
            cmd.error(
                ErrorKind::InvalidValue,
                format!("source path is not a json file, source path: {}", tmp_path),
            )
            .exit()
        }
    }

    let source_content = fs::read_to_string(source_path)?;
    // *json 默认是无序的，如果想要保持顺序那么就需要为 serde_json 添加 preserve_order 这个 features
    let source_json: Value = serde_json::from_str(&source_content)?;

    for str in target_json_path {
        let target_path = Path::new(&base_path).join(str);
        let target_path_str = path_to_normalized_str(&target_path).unwrap();

        {
            let mut cmd = Cli::command();
            if !target_path.exists() {
                cmd.error(
                    ErrorKind::InvalidValue,
                    format!("source path not exists, source path: {}", target_path_str),
                )
                .exit();
            }
            if !target_path.is_file() {
                cmd.error(
                    ErrorKind::InvalidValue,
                    format!(
                        "source path is not a file, source path: {}",
                        target_path_str
                    ),
                )
                .exit();
            }
            if target_path.extension().unwrap().ne("json") {
                cmd.error(
                    ErrorKind::InvalidValue,
                    format!(
                        "source path is not a json file, source path: {}",
                        target_path_str
                    ),
                )
                .exit();
            }
        }

        let target_content = fs::read_to_string(&target_path)?;
        let target_json: Value = serde_json::from_str(&target_content)?;

        let ret: Value = replace_with(json_path, target_json, &mut |json_pointer, _| {
            source_json.pointer(json_pointer).cloned()
        })?;

        let json_result = serde_json::to_string_pretty(&ret)?;

        fs::write(&target_path, json_result).map(|_| {
            info!("target_path: {} is finished!", target_path_str);
        })?;
    }

    Ok(())
}
