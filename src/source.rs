use git2::{Commit, DiffDelta, Repository, Revwalk, Tree};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{ffi::OsString, path::PathBuf, sync::Arc};

use crate::{data::Statistics, errors::SourceCodeError};

/// Represents the information for a specific source file during the static retrieval phase
///
/// #Fields:
/// * `name` - The name of the file
/// * `relative_path` - The relative path of the file from the root of the repository
/// * `language` - The [`LanguageType`] of the file
/// * `id_hash` - The (SHA256) hash of the file
/// * `source_file` - The contents of the file in a [`SourceFile`] container
/// * `statistics` - The [`Statistics`] on the file
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct SourceFileInfo {
    pub name: String,
    pub relative_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<LanguageType>,
    pub id_hash: Option<String>,
    #[serde(skip)]
    pub source_file: Option<Box<SourceFile>>,
    pub statistics: Statistics,
}
impl SourceFileInfo {
    pub(crate) fn set_source_file_contents(&mut self, contents: String) {
        self.source_file = Some(Box::new(SourceFile {
            parent: self.clone(),
            contents: Arc::new(contents.into()),
        }));
    }
    pub(crate) fn _get_source_file_contents(&self) -> String {
        match &self.source_file {
            Some(source_file) => source_file
                .contents
                .to_str()
                .unwrap_or_default()
                .to_string(),
            None => {
                log::error!("Failed to retrieve source file: {}", self.name);
                String::new()
            }
        }
    }
    pub(crate) fn get_source_file_info(
        source_file_path: &str,
        file_report: &tokei::Report,
        lang_type: &LanguageType,
    ) -> Result<SourceFileInfo, SourceCodeError> {
        // Get the source file contents
        let src_file_contents =
            std::fs::read_to_string(&file_report.name).map_err(SourceCodeError::FileReadError)?;
        let src_file_contents_size = Self::get_file_contents_size(&src_file_contents)?;
        let src_file_hash = Self::calculate_hash_from(&src_file_contents);

        let mut statistics =
            Statistics::get_statistics_for_source_file(source_file_path, &file_report.name)?;
        statistics.loc = file_report.stats.code as i64;
        statistics.size = src_file_contents_size;

        let mut source_file_info = SourceFileInfo {
            name: file_report
                .name
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "Name not resolved".to_string()),
            relative_path: file_report.name.as_os_str().to_string_lossy().to_string(),
            language: Some({
                let mut lang_type = lang_type.clone();
                lang_type.extensions = file_report
                    .name
                    .extension()
                    .map(|ext| {
                        ext.to_str()
                            .map(|ext_str| ext_str.to_string())
                            .unwrap_or_else(|| "Extension not resolved".to_string())
                    })
                    .into_iter()
                    .collect();
                lang_type
            }),
            id_hash: Some(src_file_hash),
            source_file: None,
            statistics,
        };
        source_file_info.set_source_file_contents(src_file_contents);

        Ok(source_file_info)
    }
    fn get_file_contents_size(file_contents: &String) -> Result<i64, SourceCodeError> {
        let length: i64 = file_contents
            .len()
            .try_into()
            .map_err(SourceCodeError::ConversionError)?;
        Ok(length)
    }
    /// Calculates a (SHA256) hash from a string (source file contents)
    fn calculate_hash_from(file_contents: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(file_contents);
        let result = hasher.finalize();

        format!("{:x}", result)
    }
}

/// Represents the contents of a source file
#[derive(Clone, Debug, PartialEq)]
pub struct SourceFile {
    parent: SourceFileInfo,
    contents: Arc<OsString>,
}

/// Top-level struct to hold statistics on the [`LanguageType`]s found in the repository.
/// Each source file will be assigned a [`LanguageType`] based on the language and file extensions.
/// Note that the "Language", e.g., 'Rust', may have multiple file extensionss, e.g., '.rs', '.toml', etc. and therefore multiple [`LanguageType`]s.
///
/// #Fields:
/// * `language` - The name of the language
/// * `extensions` - A [`Vec`] of file extensionss for this language
/// * `percentage` - The percentage of the total lines of code in the repository that are of this [`LanguageType`]
/// * `statistics` - The [`Statistics`] on the file type
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct LanguageType {
    pub name: String,
    pub extensions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statistics: Option<Statistics>,
}
impl LanguageType {
    /// Creates a new [`LanguageType`] from a tokei::LanguageType
    pub fn new_from(tokei_language_type: tokei::LanguageType) -> Self {
        Self {
            name: tokei_language_type.name().to_string(),
            extensions: vec![],
            statistics: None,
        }
    }
    /// Sums the lines of code for an array of [`LanguageType`]s
    pub fn sum_lines_of_code(language_types: &[LanguageType]) -> i64 {
        language_types
            .iter()
            .filter_map(|lt| lt.statistics.as_ref().map(|s| s.loc))
            .sum()
    }
    /// Gets the predominant language from an array of [`LanguageType`]s
    pub fn get_predominant_language(languages: &[LanguageType]) -> LanguageType {
        let mut predominant_language = LanguageType::default();
        let mut highest_percentage = 0.0;
        let mut largest_size = 0_i64;

        for lang in languages {
            if let Some(statistics) = &lang.statistics {
                if statistics.frequency > highest_percentage
                    || (statistics.frequency == highest_percentage
                        && statistics.size > largest_size)
                {
                    highest_percentage = statistics.frequency;
                    largest_size = statistics.size;
                    predominant_language = lang.clone();
                }
            }
        }
        predominant_language
    }
    /// Calculates percentage distribution of the [`LanguageType`]s - i.e., the percentage of
    /// lines of code that each [`LanguageType`] in relation to each other and updates the [`Statistics`].frequency field for each [`LanguageType`]
    pub fn calculate_percentage_distribution(languages: &mut [LanguageType]) {
        let total_lines_of_code = LanguageType::sum_lines_of_code(languages);
        for language in languages {
            if let Some(statistics) = &mut language.statistics {
                statistics.frequency = (statistics.loc as f32 / total_lines_of_code as f32) * 100.0;
            }
        }
    }
}

/// Captures the file change frequency for a file
/// #Fields:
/// * file_commits: the number of commits that the file has been changed in
/// * total_commits: the total number of commits in the repository as reference
/// * frequency: the frequency of the file being changed, as a ratio of file_commits to total_commits
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct SourceFileChangeFrequency {
    pub file_commits: i32,
    pub total_commits: i32,
    pub frequency: f32,
}
impl SourceFileChangeFrequency {
    pub fn get_as_statistics(&self) -> Statistics {
        Statistics {
            size: 0,
            loc: 0,
            num_files: 0,
            num_commits: self.file_commits,
            frequency: self.frequency,
        }
    }
    /// Gets the file change frequency for the file passed as 'source_file_path' in the repository passed as 'repo_path'
    /// #Arguments:
    /// * `repo_path` - The path to the repository
    /// * `source_file_path` - The path to the source file
    /// Returns:
    ///   - Ok([`SourceFileChangeFrequency`]) if successful
    ///   - Err([`SourceCodeError`]) if unsuccessful
    pub fn get_from_source_file(
        repo_path: &str,
        file_path: &PathBuf,
    ) -> Result<SourceFileChangeFrequency, SourceCodeError> {
        // Need to trim the 'file_path' relative to the 'repo_path'
        let repo_path_buf = PathBuf::from(repo_path);
        let file_path = PathBuf::from(file_path);
        let file_path = file_path
            .strip_prefix(repo_path_buf)
            .map_err(SourceCodeError::FilePathError)?;

        let repo: Repository = Repository::open(repo_path)?;
        let mut revwalk: Revwalk<'_> = repo.revwalk()?;
        revwalk.push_head()?;

        let mut total_commits: i32 = 0;
        let mut file_commits: i32 = 0;

        for commit_id in revwalk {
            let commit: Commit<'_> = repo.find_commit(commit_id?)?;
            total_commits += 1;

            if commit.parent_count() > 0 {
                let parent: Commit<'_> = commit.parent(0)?;
                let commit_tree: Tree<'_> = commit.tree()?;
                let parent_tree: Tree<'_> = parent.tree()?;

                let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None)?;
                diff.foreach(
                    &mut |delta: DiffDelta<'_>, _| {
                        let filepath = delta
                            .new_file()
                            .path()
                            .unwrap_or(delta.old_file().path().unwrap());
                        if filepath == file_path {
                            file_commits += 1;
                        }
                        true
                    },
                    None,
                    None,
                    None,
                )?;
            }
        }
        let frequency = file_commits as f32 / total_commits as f32 * 100.00;

        Ok(SourceFileChangeFrequency {
            file_commits,
            total_commits,
            frequency,
        })
    }
}
