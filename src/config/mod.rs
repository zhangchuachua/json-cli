use crate::error::CliError;
use crate::error::CliError::{InvalidArrPath, InvalidIndex};
use crate::util::{format_to_by_regexp, get_matched_file_paths};
use fs_extra::dir::create_all;
use fs_extra::file::{copy, CopyOptions};
use log::warn;
use serde_json::map::Entry;
use serde_json::Value;
use serde_json_path::JsonPath;
use std::fmt::{Display, Formatter};
use std::path::{Component, Path, PathBuf};

pub struct Config {
    matched_paths: Vec<String>,
    from: String,
    to: String,
    options: CopyOptions,
}

impl Config {
    pub fn new(base_path: String, from: String, to: String, options: CopyOptions) -> Self {
        let from_path = Path::new(&base_path).join(from).normalize();
        let normalized_to = Path::new(&base_path)
            .join(to)
            .normalize()
            .to_string_lossy()
            .to_string();
        let matched_paths = get_matched_file_paths(&from_path).unwrap();

        Self {
            matched_paths,
            from: from_path.to_string_lossy().to_string(),
            to: normalized_to,
            options,
        }
    }

    pub fn copy_file(&self) -> Result<(), CliError> {
        for path in &self.matched_paths {
            let to = format_to_by_regexp(&self.from, &self.to, path);
            let mut to = PathBuf::from(&to);

            if to.extension().is_none() {
                // 如果没有 extension 那么认为当前的 to 是文件夹，那么就使用 path 的文件名;
                let file_name = Path::new(path).file_name().unwrap();
                to = to.join(file_name);
            }
            // 到这里 to 一定包含文件名了，不然会直接抛出 panic
            let parent = to.parent().unwrap();
            // 创建不存在的路径
            if !parent.exists() {
                create_all(parent, false)?;
            }
            copy(path, &to, &self.options).map(|status| {
                if status == 0 {
                    println!("{} 已存在, 将跳过", to.display());
                } else {
                    println!("{} --> {} success!", path, to.display());
                }
            })?;
        }

        Ok(())
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "from: {}\n to: {}\n matched_paths: {:#?}",
            self.from, self.to, self.matched_paths
        )
    }
}

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

trait Normalize {
    fn normalize(&self) -> Self;
}

impl Normalize for PathBuf {
    fn normalize(&self) -> Self {
        let mut tmp = PathBuf::new();
        for component in self.components() {
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
}
