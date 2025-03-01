use crate::core::copy::CopyService;
use crate::core::JsonService;
use crate::error::AppError;
use std::path::PathBuf;

pub struct CommandRunner {}

impl CommandRunner {
    pub fn new() -> Self {
        Self {}
    }

    pub fn copy(
        &self,
        from: PathBuf,
        to: PathBuf,
        skip_exist: bool,
        ignore_dirs: Option<Vec<String>>,
    ) -> Result<(), AppError> {
        CopyService::new(from, to, skip_exist, ignore_dirs).copy_file_to_target()
    }

    pub fn modify_json(
        &self,
        from: PathBuf,
        to: PathBuf,
        json_path: String,
        skip_exist: bool,
    ) -> Result<(), AppError> {
        JsonService::new(from, to, json_path, skip_exist).modify_json()
    }
}
