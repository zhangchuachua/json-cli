use fs_extra::file::CopyOptions;
use inquire::{validator::Validation, Confirm, Select, Text};
use jh::config::Config;
use std::{fmt, path::Path};
use jh::file_path_completer::FilePathCompleter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 在代码中设置 log 等级，kt 是当前的包名，意味开启所有级别的日志打印；
    std::env::set_var("RUST_LOG", "jh");
    env_logger::init();

    let command = Select::new(
        "请选择你要执行的命令",
        vec![Command::CopyFileToTarget, Command::CopyJsonPathToTarget],
    )
    .prompt()?;

    let base_path = Text::new("请输入基础路径: ")
        .with_validator(|s: &str| {
            // canonicalize 将会判断是否是目录，是否存在
            match Path::new(s).canonicalize() {
                Ok(_) => Ok(Validation::Valid),
                Err(_) => Ok(Validation::Invalid("路径不存在或不是目录".into())),
            }
        })
        .with_autocomplete(FilePathCompleter::default())
        .with_default("")
        .prompt()?;

    match command {
        Command::CopyFileToTarget => {
            //* cp 命令
            let source_path = Text::new("请输入源文件路径: ").with_autocomplete(FilePathCompleter::default().base(base_path.clone())).prompt()?;
            let target_path = Text::new("请输入目标路径: ").with_autocomplete(FilePathCompleter::default().base(base_path.clone())).prompt()?;
            let skip_exist = Confirm::new("是否跳过已存在的文件？(default: true)")
                .with_default(true)
                .prompt()?;
            let overwrite = Confirm::new("是否覆盖已存在的文件？(default: false)")
                .with_default(false)
                .prompt()?;
            let options = CopyOptions::new()
                .skip_exist(skip_exist)
                .overwrite(overwrite);

            let config = Config::new(base_path, source_path, target_path, options);
            config.copy_file().map(|_| {
                println!("copy success!");
            })?;
        }

        Command::CopyJsonPathToTarget => {
            todo!()
        }
    }

    Ok(())
}

#[derive(Debug)]
enum Command {
    CopyFileToTarget,
    CopyJsonPathToTarget,
}
impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::CopyFileToTarget => write!(f, "复制源文件到指定位置，支持正则，例如: ./(?<lang>[a-z-]{{2,}}/.*\\.json) -> ./other/$lang/"),
            Command::CopyJsonPathToTarget => write!(f, "复制 json_path 对应的内容到指定文件; 例如： en/home.json 下的 $.meta.* -> de/home.json; 语法请见：https://docs.rs/serde_json_path/0.7.1/serde_json_path/"),
        }
    }
}

// fn cp_json_path(
//     source: &str,
//     json_path: &str,
//     target_json_path: &Vec<String>,
//     base_path: &Option<String>,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     let base_path = base_path.clone().unwrap_or_default();
//     let source_path = Path::new(&base_path).join(source);
//
//     {
//         let mut cmd = Cli::command();
//         let tmp_path = path_to_normalized_str(&source_path).unwrap();
//         if !source_path.exists() {
//             cmd.error(
//                 ErrorKind::InvalidValue,
//                 format!("source path not exists, source path: {}", tmp_path),
//             )
//             .exit();
//         }
//         if !source_path.is_file() {
//             cmd.error(
//                 ErrorKind::InvalidValue,
//                 format!("source path is not a file, source path: {}", tmp_path),
//             )
//             .exit();
//         }
//         if source_path.extension().unwrap().ne("json") {
//             cmd.error(
//                 ErrorKind::InvalidValue,
//                 format!("source path is not a json file, source path: {}", tmp_path),
//             )
//             .exit()
//         }
//     }
//
//     let source_content = fs::read_to_string(source_path)?;
//     // *json 默认是无序的，如果想要保持顺序那么就需要为 serde_json 添加 preserve_order 这个 features
//     let source_json: Value = serde_json::from_str(&source_content)?;
//
//     for str in target_json_path {
//         let target_path = Path::new(&base_path).join(str);
//         let target_path_str = path_to_normalized_str(&target_path).unwrap();
//
//         {
//             let mut cmd = Cli::command();
//             if !target_path.exists() {
//                 cmd.error(
//                     ErrorKind::InvalidValue,
//                     format!("source path not exists, source path: {}", target_path_str),
//                 )
//                 .exit();
//             }
//             if !target_path.is_file() {
//                 cmd.error(
//                     ErrorKind::InvalidValue,
//                     format!(
//                         "source path is not a file, source path: {}",
//                         target_path_str
//                     ),
//                 )
//                 .exit();
//             }
//             if target_path.extension().unwrap().ne("json") {
//                 cmd.error(
//                     ErrorKind::InvalidValue,
//                     format!(
//                         "source path is not a json file, source path: {}",
//                         target_path_str
//                     ),
//                 )
//                 .exit();
//             }
//         }
//
//         let target_content = fs::read_to_string(&target_path)?;
//         let target_json: Value = serde_json::from_str(&target_content)?;
//
//         let ret: Value = replace_with(json_path, target_json, &mut |json_pointer, _| {
//             source_json.pointer(json_pointer).cloned()
//         })?;
//
//         let json_result = serde_json::to_string_pretty(&ret)?;
//
//         fs::write(&target_path, json_result).map(|_| {
//             info!("target_path: {} is finished!", target_path_str);
//         })?;
//     }
//
//     Ok(())
// }
