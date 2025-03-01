use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 解析错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("路径无效: {0}")]
    InvalidPath(String),

    #[error("JSON 路径解析错误 `{0}`")]
    JsonPathParse(String),

    #[error("无效对象路径: {0}")]
    InvalidObjectPath(String),

    #[error("无效数组路径: {0}")]
    InvalidArrayPath(String),

    #[error("无效索引: {0}")]
    InvalidIndex(String),

    #[error("文件操作错误: {0}")]
    FileOperation(#[from] fs_extra::error::Error),

    #[error("目录操作错误: {0}")]
    DirectoryError(String),
}
