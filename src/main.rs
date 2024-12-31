use std::{
    fs::{self},
    path::{Component, Path, PathBuf},
};

use clap::{error::ErrorKind, ColorChoice, CommandFactory, Parser, Subcommand};
use colored::Colorize;
use log::{error, info, warn};
use serde_json::{map::Entry, Value};
use serde_json_path::JsonPath;

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
    std::env::set_var("RUST_LOG", "kt");
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
    let base_path = base_path.clone().unwrap_or_else(|| "".to_string());

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
            target_path = target_path.join(&source_path.file_name().unwrap());
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

fn normalize_path(path: &Path) -> PathBuf {
    let mut tmp = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                tmp.pop();
            }
            Component::CurDir => {}
            _ => {
                tmp.push(component);
            }
        }
    }
    tmp
}

fn path_to_normalized_str(path: &Path) -> Option<String> {
    let tmp = normalize_path(path);
    tmp.to_str().map(|s| s.to_string())
}

fn cp_json_path(
    source: &str,
    json_path: &str,
    target_json_path: &Vec<String>,
    base_path: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_path = base_path.clone().unwrap_or_else(|| "".to_string());

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
        let target_content = fs::read_to_string(&target_path)?;
        let target_json: Value = serde_json::from_str(&target_content)?;
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

        let ret: Value = replace_with(json_path, target_json, &mut |json_pointer, _| {
            if let Some(v) = source_json.pointer(json_pointer) {
                Some(v.clone())
            } else {
                None
            }
        })?;

        let json_result = serde_json::to_string_pretty(&ret)?;

        fs::write(&target_path, json_result).map(|_| {
            info!("target_path: {} is finished!", target_path_str);
        })?;
    }

    Ok(())
}

fn replace_with<F: FnMut(&str, Value) -> Option<Value>>(
    replace_path: &str,
    mut value: Value,
    func: &mut F,
) -> Result<Value, clap::error::Error> {
    let mut cmd = Cli::command();
    // replace_path = "$.meta.*";
    let json_path = JsonPath::parse(replace_path).map_err(|e| {
        cmd.error(
            ErrorKind::ValueValidation,
            format!(
                "This path ({}) is not a valid path; \n Parse msg: {}",
                replace_path.green(),
                e
            ),
        )
    })?;

    // pointers = ["/meta/title", "/meta/desc", "/meta/keywords"];
    let pointers = json_path
        .query_located(&value)
        .locations()
        // *转换成 json_pointer 后就把 query_located 消费了，对 value 的不可变借用也就消费了，所以后面可以对 value 可变借用；
        .map(|l| l.to_json_pointer())
        .collect::<Vec<String>>();

    // "/meta/title"
    for pointer in pointers {
        // ["meta", "title"]
        let path_vec = pointer
            .split("/")
            .filter(|v| !v.is_empty())
            .collect::<Vec<&str>>();

        let last_index = path_vec.len().saturating_sub(1);

        let mut target = &mut value;

        for index in 0..path_vec.len() {
            // meta, title
            let key = path_vec[index];
            let is_last = index == last_index;
            let target_once = target;

            let target_next = match *target_once {
                Value::Object(ref mut obj) => {
                    if is_last {
                        if let Entry::Occupied(mut e) = obj.entry(key) {
                            let v = e.insert(Value::Null);
                            if let Some(res) = func(&pointer, v) {
                                e.insert(res);
                            } else {
                                e.remove();
                            }
                        } else {
                            warn!("这个路径 ({}) 将会获取到空 entry, 跳过该路径！", &pointer);
                        }
                        None
                    } else {
                        if let Some(tmp) = obj.get_mut(key) {
                            Some(tmp)
                        } else {
                            // TODO 对象不存在时是否新建一个对象
                            return Err(cmd.error(
                                ErrorKind::InvalidValue,
                                format!("这个路径 ({}) 无法获取到正确的对象!", &pointer),
                            ));
                        }
                    }
                }
                Value::Array(ref mut arr) => {
                    // 将 key 解析为 usize 如果不能解析为 usize 说明 key 有误，所以返回 None 跳过
                    if let Ok(i) = key.parse::<usize>() {
                        if is_last {
                            let v = std::mem::replace(&mut arr[i], Value::Null);
                            if let Some(res) = func(&pointer, v) {
                                arr[i] = res;
                            } else {
                                arr.remove(i);
                            }
                            None
                        } else {
                            if let Some(tmp) = arr.get_mut(i) {
                                Some(tmp)
                            } else {
                                // TODO 对象不存在时是否新建一个对象
                                return Err(cmd.error(
                                    ErrorKind::InvalidValue,
                                    format!("这个路径 ({}) 无法获取到正确的数组!", &pointer),
                                ));
                            }
                        }
                    } else {
                        return Err(cmd.error(
                            ErrorKind::InvalidValue,
                            format!(
                                "这个路径({:?})中的索引无法解析为 usize 导致无法获取到正确的数组!",
                                &pointer
                            ),
                        ));
                    }
                }
                _ => None,
            };

            if let Some(ret) = target_next {
                target = ret;
            } else {
                break;
            }
        }
    }

    Ok(value)
}

#[test]
fn test() {}
