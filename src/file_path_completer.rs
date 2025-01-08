use crate::error::CliError;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use inquire::autocompletion::Replacement;
use inquire::{Autocomplete, CustomUserError};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

// source : https://github.com/mikaelmello/inquire/blob/main/inquire/examples/complex_autocompletion.rs
#[derive(Clone, Default)]
pub struct FilePathCompleter {
    base: String,
    input: String,
    paths: Vec<String>,
}

impl FilePathCompleter {
    pub fn base(mut self, base: String) -> Self {
        self.base = base;
        self
    }
    fn update_input(&mut self, input: &str) -> Result<(), CliError> {
        if input == self.input && !self.paths.is_empty() {
            return Ok(());
        }

        self.input = input.to_owned();
        self.paths.clear();

        let input_path = Path::new(&self.base).join(input);

        let fallback_parent = input_path
            .parent()
            .map(|p| {
                if p.to_string_lossy() == "" {
                    PathBuf::from(".")
                } else {
                    p.to_owned()
                }
            })
            .unwrap_or_else(|| PathBuf::from("."));

        let scan_dir = if input.ends_with('/') {
            input_path
        } else {
            fallback_parent.clone()
        };

        let entries = match std::fs::read_dir(scan_dir) {
            Ok(read_dir) => Ok(read_dir),
            Err(err) if err.kind() == ErrorKind::NotFound => std::fs::read_dir(fallback_parent),
            Err(err) => Err(err),
        }?
            .collect::<Result<Vec<_>, _>>()?;

        for entry in entries {
            let path = entry.path();
            let path_str = if path.is_dir() {
                format!("{}/", path.to_string_lossy())
            } else {
                path.to_string_lossy().to_string()
            };

            self.paths.push(path_str);
        }

        Ok(())
    }

    fn fuzzy_sort(&self, input: &str) -> Vec<(String, i64)> {
        let mut matches: Vec<(String, i64)> = self
            .paths
            .iter()
            .filter_map(|path| {
                SkimMatcherV2::default()
                    .smart_case()
                    .fuzzy_match(path, input)
                    .map(|score| (path.clone(), score))
            })
            .collect();

        matches.sort_by(|a, b| b.1.cmp(&a.1));
        matches
    }
}

impl Autocomplete for FilePathCompleter {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        self.update_input(input)?;

        let matches = self.fuzzy_sort(input);
        Ok(matches.into_iter().take(15).map(|(path, _)| path).collect())
    }

    fn get_completion(
        &mut self,
        input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<Replacement, CustomUserError> {
        self.update_input(input)?;

        let base_path = Path::new(&self.base);
        Ok(if let Some(suggestion) = highlighted_suggestion {
            let result: String = Path::new(&suggestion).strip_prefix(base_path).unwrap().to_string_lossy().to_string();
            Replacement::Some(result)
        } else {
            let matches = self.fuzzy_sort(input);
            matches
                .first()
                .map(|(path, _)| {
                    let result: String = Path::new(&path).strip_prefix(base_path).unwrap().to_string_lossy().to_string();
                    Replacement::Some(result)
                })
                .unwrap_or(Replacement::None)
        })
    }
}
