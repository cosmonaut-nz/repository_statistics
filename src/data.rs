use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{errors::SourceCodeError, source::SourceFileChangeFrequency};

/// Struct to hold statistics on the code in a repository
///
/// # Fields:
/// * `size` - The size of the repository in bytes
/// * `loc` - The number of lines of code in the repository
/// * `num_file` - The number of files in the repository
/// * `num_commits` - The number of commits in the repository
/// * `frequency` - The frequency of commits to the repository, as a ratio of commits to total commits in the repository
#[derive(Clone, Default, Serialize, Deserialize, Debug, PartialEq)]
pub struct Statistics {
    pub size: i64,
    pub loc: i64,
    pub num_files: i32,
    pub num_commits: i32,
    pub frequency: f32,
}
impl Statistics {
    pub fn new() -> Self {
        Self {
            size: 0,
            loc: 0,
            num_files: 0,
            num_commits: 0,
            frequency: 0.0,
        }
    }
    /// Gets a [`Statistics`] struct for a given source file path
    pub fn get_statistics_for_source_file(
        repo_path: &str,
        source_file_path: &PathBuf,
    ) -> Result<Self, SourceCodeError> {
        let scf = SourceFileChangeFrequency::get_from_source_file(repo_path, source_file_path)?;

        Ok(Self {
            size: 0, // Should be sourced from tokei
            loc: 0,  // Should be sourced from tokei
            num_files: 1,
            num_commits: scf.file_commits,
            frequency: scf.frequency,
        })
    }
}
