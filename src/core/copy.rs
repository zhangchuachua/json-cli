use crate::error::AppError;
use crate::util::{format_to_path_by_regexp, get_matched_file_paths, Normalize};
use fs_extra::dir::create_all;
use fs_extra::file::{copy, CopyOptions};
use log::{info, warn};
use std::path::{Path, PathBuf};

pub struct CopyService {
    from: PathBuf,
    to: PathBuf,
    matched_paths: Vec<String>,
    ignore_dirs: Option<Vec<String>>,
    options: CopyOptions,
}

impl CopyService {
    pub fn new(
        from: PathBuf,
        to: PathBuf,
        skip_exist: bool,
        ignore_dirs: Option<Vec<String>>,
    ) -> Self {
        let from = from.normalize();
        let to = to.normalize();
        let matched_paths = get_matched_file_paths(&from, &ignore_dirs).unwrap();
        let options = CopyOptions::new().overwrite(true).skip_exist(skip_exist);
        Self {
            from,
            to,
            matched_paths,
            ignore_dirs,
            options,
        }
    }

    pub fn copy_file_to_target(&self) -> Result<(), AppError> {
        for path in &self.matched_paths {
            let to = format_to_path_by_regexp(
                &self.from.to_string_lossy(),
                &self.to.to_string_lossy(),
                path.as_str(),
            );
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
            copy(&path, &to, &self.options)
                .map(|status| {
                    if status == 0 {
                        warn!("{} 已存在, 将跳过", to.display());
                    } else {
                        info!("{} --> {} success!", path, to.display());
                    }
                })
                .unwrap();
        }
        Ok(())
    }
}
