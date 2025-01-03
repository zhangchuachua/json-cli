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

// TODO 完善 option 功能，比如覆写
fn copy_file(from: &str, to: &str) -> Result<(), Box<dyn std::error::Error>> {
    let to_parent = Path::new(to).parent().unwrap();
    if !to_parent.exists() {
        create_all(to_parent, false)?;
    }

    let options = CopyOptions::new();
    copy(from, to, &options)?;

    Ok(())
}

const IGNORE_FILE: [&str; 1] = [".DS_Store"];

fn get_matched_file_paths(from: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let re_path = normalize_path(&Path::new(from));
    let mut matched_paths = vec![PathBuf::new()];

    for cpt in re_path.components() {
        let mut tmp_paths = vec![];
        if matched_paths.is_empty() {
            panic!(
                "`{}`
                 路径错误; 可能是路径中有不存在的文件夹导致!",
                re_path.display()
            );
        }
        for path in matched_paths {
            let new_path = path.join(cpt);
            // 如果 path 是存在的，就不去处理它
            if new_path.exists() {
                tmp_paths.push(new_path);
                continue;
            }
            // 如果 path 不存在，那么认为这个 cpt 是一个正则表达式；
            let cpt_to_str = cpt.as_os_str().to_str().unwrap();
            // 使用这个 path 构建正则表达式;
            let rex = Regex::new(cpt_to_str).unwrap();

            if path.is_file() {
                panic!(
                    "`{}` 路径错误; 可能是路径中间有文件导致!",
                    re_path.display()
                );
            }

            // 这里的 path 相当于 new_path 的 parent 如果 new_path 不存在，那么就去读取其 parent
            // 如果 path 是一个目录的话，读取这个目录，并寻找匹配正则表达式的 file_name 并添加到 path_vec 中
            path.read_dir()?
                .filter_map(|item| {
                    item.ok().and_then(|dir_entry| {
                        dir_entry
                            .file_name()
                            .into_string()
                            .ok()
                            .and_then(|file_name| {
                                // 过滤隐藏文件
                                if IGNORE_FILE.contains(&file_name.as_str()) {
                                    None
                                    // 过滤不匹配的文件
                                } else if rex.is_match(&file_name) {
                                    Some(file_name)
                                } else {
                                    None
                                }
                            })
                    })
                })
                .for_each(|file_name| {
                    tmp_paths.push(path.join(file_name));
                })
        }
        matched_paths = tmp_paths;
    }

    Ok(matched_paths
        .iter()
        .map(|item| item.to_str().unwrap().to_string())
        .collect::<Vec<String>>())
}

fn get_target_paths_from_regexp(
    matched_paths: &Vec<String>,
    from: &str,
    to: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let re_path = normalize_path(&Path::new(from));
    let rex = Regex::new(re_path.to_str().unwrap()).unwrap();
    let mut target_paths = vec![];
    for path in matched_paths {
        target_paths.push(rex.replace_all(&path, to).to_string());
    }

    Ok(target_paths)
}
