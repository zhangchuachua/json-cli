use clap::Parser;
use my_helper::cli::{Cli, CommandRunner, Commands};
use my_helper::error::AppError;

fn main() -> Result<(), AppError> {
    // 初始化日志
    std::env::set_var("RUST_LOG", "trace");
    env_logger::init();

    let cli = Cli::parse();
    let runner = CommandRunner::new();

    match cli.command {
        Commands::Copy {
            from,
            to,
            skip_exist,
        } => runner.copy(from, to, skip_exist, cli.ignore_dirs),
        Commands::ModifyJson {
            from,
            to,
            json_path,
            skip_exist,
        } => runner.modify_json(from, to, json_path, skip_exist),
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
