use crate::error::CliError;
use crate::error::CliError::{InvalidArrPath, InvalidIndex};
use log::warn;
use regex::Regex;
use serde_json::map::Entry;
use serde_json::Value;
use serde_json_path::JsonPath;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

/// 从 from 中获取匹配到的路径，例如 "/Users/xxx/Desktop/(?<name>.*)\.png" 表示匹配桌面下的所有 png
pub fn get_matched_file_paths(
    from: &Path,
    ignore_dirs: Option<Vec<String>>,
) -> Result<Vec<String>, CliError> {
    let real_ignore = match ignore_dirs {
        Some(value) => value,
        None => vec![
            ".DS_Store".to_string(),
            ".git".to_string(),
            ".idea".to_string(),
            ".vscode".to_string(),
        ],
    };
    // set 过滤重复的路径； 注意 set 是无序的；
    let mut matched_paths = HashSet::new();
    matched_paths.insert(PathBuf::new());

    for cpt in from.components() {
        let mut tmp_paths = HashSet::new();
        if matched_paths.is_empty() {
            panic!(
                "`{}` 路径错误; 可能是路径中有不存在的文件夹导致!",
                from.display()
            );
        }
        for path in matched_paths {
            let new_path = path.join(cpt);
            // 如果 path 是存在的，就不去处理它
            if new_path.exists() {
                tmp_paths.insert(new_path);
                continue;
            }
            // 如果 path 不存在，那么认为这个 cpt 是一个正则表达式；
            let cpt_to_str = cpt.as_os_str().to_str().unwrap();
            // 使用这个 path 构建正则表达式;
            let rex = Regex::new(cpt_to_str).unwrap();

            if path.is_file() {
                panic!(
                    "在 `{}` 出遇到问题，可能是路径中间有文件导致, 完整匹配为 `{}`",
                    path.display(),
                    from.display()
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
                                if real_ignore.contains(&file_name) {
                                    None
                                } else if rex.find(&file_name)?.as_str() == file_name {
                                    // 过滤不匹配的文件 注意，这里的规则是必须完全匹配整个 file_name 相当于规定了 ^ 开头 和 $ 结尾
                                    Some(file_name)
                                } else {
                                    None
                                }
                            })
                    })
                })
                .for_each(|file_name| {
                    tmp_paths.insert(path.join(file_name));
                })
        }
        matched_paths = tmp_paths;
    }

    Ok(matched_paths
        .iter()
        .map(|item| item.to_str().unwrap().to_string())
        .collect::<Vec<String>>())
}

/// 使用 regexp 格式化 to path 例如： from_rex = "/Users/xxx/Desktop/(?<name>.*)\.png" to_rex = "/Users/xxx/$name.png" path = "/Users/xxx/Desktop/rust.png" 得到结果为 "/Users/xxx/rust.png"
pub fn format_to_path_by_regexp(from_rex: &str, to_rex: &str, path: &str) -> String {
    let rex = Regex::new(from_rex).unwrap();
    let mut result = String::new();
    let caps = rex.captures(path).unwrap();
    caps.expand(to_rex, &mut result);
    result
}

/// 替换 json 中的指定路径；
pub fn replace_with<F: FnMut(&str, Value) -> Option<Value>>(
    replace_path: &str,
    mut value: Value,
    func: &mut F,
) -> Result<Value, CliError> {
    // replace_path = "$.meta.*";
    let json_path = JsonPath::parse(replace_path)
        .map_err(|_| CliError::JsonPathParseError(replace_path.to_string()))?;

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
                    } else if let Some(tmp) = obj.get_mut(key) {
                        Some(tmp)
                    } else {
                        // TODO 对象不存在时是否新建一个对象
                        return Err(CliError::InvalidObjPath(pointer.to_string()));
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
                        } else if let Some(tmp) = arr.get_mut(i) {
                            Some(tmp)
                        } else {
                            // TODO 对象不存在时是否新建一个对象
                            return Err(InvalidArrPath(pointer.to_string()));
                        }
                    } else {
                        return Err(InvalidIndex(key.to_string()));
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

// 将 "/Users/xx/Desktop/example/../json-cli"  这样包含 `../` `./` 的路径格式化为普通的路径
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut tmp = PathBuf::new();
    for component in path.components() {
        println!("{}", component.as_os_str().to_str().unwrap());
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

pub fn path_to_normalized_str(path: &Path) -> Option<String> {
    normalize_path(path).to_str().map(|s| s.to_string())
}
