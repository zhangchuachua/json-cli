use crate::util::{format_to_path_by_regexp, get_matched_file_paths};
use fs_extra::dir::create_all;
use fs_extra::file::{copy, CopyOptions};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
struct CustomOptions {
    /// Sets the option true for overwrite existing files.
    pub overwrite: bool,
    /// Sets the option true for skip existing files.
    pub skip_exist: bool,
    pub ignore_dirs: Option<Vec<String>>,
}

#[wasm_bindgen]
pub fn copy_file(from: String, to: String, options: String) {
    // 因为 wasm_bindgen 要求 struct 中的值都必须是实现了 Copy 的，所以能设置的属性非常有限
    // 于是 options 使用 json 字符串来代替
    let typed_options: CustomOptions = serde_json::from_str(&options).unwrap();
    let copy_options = CopyOptions::new()
        .overwrite(typed_options.overwrite)
        .skip_exist(typed_options.skip_exist);

    let from_path = Path::new(&from).to_path_buf().normalize();
    let to_path = Path::new(&to).to_path_buf().normalize();

    let from_string = from_path.to_string_lossy().to_string();
    let to_string = to_path.to_string_lossy().to_string();

    let matched_paths = get_matched_file_paths(&from_path, typed_options.ignore_dirs).unwrap();

    for path in matched_paths {
        let to = format_to_path_by_regexp(&from_string, &to_string, &path);
        let mut to = PathBuf::from(&to);

        if to.extension().is_none() {
            // 如果没有 extension 那么认为当前的 to 是文件夹，那么就使用 path 的文件名;
            let file_name = Path::new(&path).file_name().unwrap();
            to = to.join(file_name);
        }
        // 到这里 to 一定包含文件名了，不然会直接抛出 panic
        let parent = to.parent().unwrap();
        // 创建不存在的路径
        if !parent.exists() {
            create_all(parent, false).unwrap();
        }
        copy(&path, &to, &copy_options)
            .map(|status| {
                if status == 0 {
                    println!("{} 已存在, 将跳过", to.display());
                } else {
                    println!("{} --> {} success!", path, to.display());
                }
            })
            .unwrap();
    }
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
