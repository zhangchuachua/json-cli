use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("JsonPath 解析错误 `{0}`!")]
    JsonPathParseError(String),
    #[error("这个路径 `{0}` 无法获取到正确的对象!")]
    InvalidObjPath(String),
    #[error("这个路径 `{0}` 无法获取到正确的数组!")]
    InvalidArrPath(String),
    #[error("错误的索引 `{0}`!")]
    InvalidIndex(String),
    #[error("文件复制发生错误 `{0}`")]
    CopyFileError(#[from] fs_extra::error::Error),
    #[error("文件夹读取错误 `{0}`")]
    ReadDirError(#[from] io::Error),
}
