use crate::error::AppError;
use crate::util::Normalize;
use std::path::PathBuf;

pub struct JsonService {
    from: PathBuf,
    to: PathBuf,
    json_path: String,
    skip_exist: bool,
}

impl JsonService {
    pub fn new(from: PathBuf, to: PathBuf, json_path: String, skip_exist: bool) -> Self {
        Self {
            from: from.normalize(),
            to: to.normalize(),
            json_path,
            skip_exist,
        }
    }

    pub fn modify_json(&self) -> Result<(), AppError> {
        todo!("modify json");
    }
}
