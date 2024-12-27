use std::{
    borrow::BorrowMut,
    fs::{self},
    path::{Component, Path, PathBuf},
};

use clap::{error::ErrorKind, CommandFactory, Parser, Subcommand};
use log::{error, warn};
use serde_json::{json, map::Entry, Value};
use serde_json_path::JsonPath;

#[derive(Parser)]
#[command(name = "File Copier", version, about, long_about = None)]
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
            cp_json_path(source, json_path, target, base_path);
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

fn cp_json_path(source: &str, json_path: &str, _: &Vec<String>, base_path: &Option<String>) {
    let base_path = base_path.clone().unwrap_or_else(|| "".to_string());

    let source_path = Path::new(&base_path).join(source);

    {
        let mut cmd = Cli::command();
        let formated_source_path = normalize_path(&source_path);
        let displayed_source_path = formated_source_path.display();
        if !source_path.exists() {
            cmd.error(
                ErrorKind::InvalidValue,
                format!(
                    "source path not exists, source path: {}",
                    displayed_source_path
                ),
            )
            .exit();
        }
        if !source_path.is_file() {
            cmd.error(
                ErrorKind::InvalidValue,
                format!(
                    "source path is not a file, source path: {}",
                    displayed_source_path
                ),
            )
            .exit();
        }
        if source_path.extension().unwrap().ne("json") {
            cmd.error(
                ErrorKind::InvalidValue,
                format!(
                    "source path is not a json file, source path: {}",
                    displayed_source_path
                ),
            )
            .exit();
        }
    }

    let source_content = fs::read_to_string(source_path).unwrap();
    let mut source_json: Value = serde_json::from_str(&source_content).unwrap();
}

fn replace_with<F: FnMut(&Vec<&str>, Value) -> Option<Value>>(
    replace_path: &str,
    mut value: Value,
    mut func: F,
) -> Value {
    // replace_path = "$.meta.*";
    let json_path = JsonPath::parse(replace_path).expect("valid JSON Path");
    // locs = ["/meta/title", "/meta/desc", "/meta/keywords"];
    let pointers = json_path
        .query_located(&value)
        .locations()
        .map(|l| l.to_json_pointer()) // 转换成 json_pointer 后就把 query_located 消费了，对 value 的不可变借用也就消费了，所以后面可以对 value 可变借用；
        .collect::<Vec<String>>();

    for pointer in pointers {
        // ["meta", "title"]
        let path_vec = pointer
            .split("/")
            .filter(|v| !v.is_empty())
            .collect::<Vec<&str>>();

        let last_index = path_vec.len().saturating_sub(1);

        let mut target = &mut value;

        for index in 0..path_vec.len() {
            let key = path_vec[index];
            let is_last = index == last_index;
            let target_once = target;

            let target_next = match *target_once {
                Value::Object(ref mut obj) => {
                    if is_last {
                        if let Entry::Occupied(mut e) = obj.entry(key) {
                            let v = e.insert(Value::Null);
                            if let Some(res) = func(&path_vec, v) {
                                e.insert(res);
                            } else {
                                e.remove();
                            }
                        } else {
                            warn!("这个路径 ({:?}) 将会获取到空 entry 跳过", path_vec);
                        }
                        None
                    } else {
                        if let Some(tmp) = obj.get_mut(key) {
                            Some(tmp)
                        } else {
                            // TODO 对象不存在时是否新建一个对象
                            error!("这个路径 ({:?}) 无法获取到正确的对象!", path_vec);
                            panic!();
                        }
                    }
                }
                Value::Array(ref mut arr) => {
                    // 将 key 解析为 usize 如果不能解析为 usize 说明 key 有误，所以返回 None 跳过
                    if let Ok(i) = key.parse::<usize>() {
                        if is_last {
                            let v = std::mem::replace(&mut arr[i], Value::Null);
                            if let Some(res) = func(&path_vec, v) {
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
                                error!("这个路径 ({:?}) 无法获取到正确的数组!", path_vec);
                                panic!();
                            }
                        }
                    } else {
                        error!(
                            "这个路径({:?})中的索引无法解析为 usize 导致无法获取到正确的数组!",
                            path_vec
                        );
                        panic!();
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

    value
}

#[test]
fn test() {}
