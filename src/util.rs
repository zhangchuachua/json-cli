use crate::error::CliError;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

const IGNORE_FILE: [&str; 4] = [".DS_Store", ".git", ".idea", ".vscode"];

pub fn get_matched_file_paths(from: &Path) -> Result<Vec<String>, CliError> {
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
                                if IGNORE_FILE.contains(&file_name.as_str()) {
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

pub fn format_to_by_regexp(from_rex: &str, to_rex: &str, path: &str) -> String {
    let rex = Regex::new(from_rex).unwrap();
    let mut result = String::new();
    let caps = rex.captures(path).unwrap();
    caps.expand(to_rex, &mut result);
    result
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
    normalize_path(path).to_str().map(|s| s.to_string())
}
