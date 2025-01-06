use crate::error::CliError;
use crate::error::CliError::{InvalidArrPath, InvalidIndex};
use fs_extra::dir::create_all;
use fs_extra::file::{copy, CopyOptions};
use log::warn;
use regex::Regex;
use serde_json::map::Entry;
use serde_json::Value;
use serde_json_path::JsonPath;
use std::path::{Component, Path, PathBuf};

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

// TODO 完善 option，比如覆写
pub fn copy_file(from: &str, to: &str) -> Result<(), CliError> {
    let to_parent = Path::new(to).parent().unwrap();
    if !to_parent.exists() {
        create_all(to_parent, false)?;
    }

    let options = CopyOptions::new();
    copy(from, to, &options)?;

    Ok(())
}

const IGNORE_FILE: [&str; 1] = [".DS_Store"];

pub fn get_matched_file_paths(from: &str) -> Result<Vec<String>, CliError> {
    let re_path = normalize_path(Path::new(from));
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

pub fn get_target_paths_from_regexp(
    matched_paths: &Vec<String>,
    from: &str,
    to: &str,
) -> Result<Vec<String>, CliError> {
    let re_path = normalize_path(Path::new(from));
    let rex = Regex::new(re_path.to_str().unwrap()).unwrap();
    let mut target_paths = vec![];
    for path in matched_paths {
        target_paths.push(rex.replace_all(path, to).to_string());
    }

    Ok(target_paths)
}

pub fn normalize_path(path: &Path) -> PathBuf {
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

pub fn path_to_normalized_str(path: &Path) -> Option<String> {
    let tmp = normalize_path(path);
    tmp.to_str().map(|s| s.to_string())
}
