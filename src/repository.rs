use chrono::{DateTime, NaiveDateTime, Utc};
use git2::{Commit, Repository, Revwalk};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokei::{Config, Languages};

use crate::{
    data::Statistics,
    errors::SourceCodeError,
    source::{LanguageType, SourceFileInfo},
};

/// Represents the information for a software source repository (Git)
///
/// #Fields:
/// * `name` - The name of the repository
/// * `predominant_language` - The [`LanguageType`] of the repository
/// * `statistics` - The [`Statistics`] on the repository
/// * `contributors` - The [`Contributor`]s to the repository
/// * `source_files` - The [`SourceFileInfo`]s for the source files of the repository
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct RepositoryInfo {
    pub name: String,
    pub predominant_language: Option<LanguageType>,
    pub statistics: Statistics,
    pub contributors: Vec<Contributor>,
    pub source_files: Vec<SourceFileInfo>,
}
impl RepositoryInfo {
    pub fn new(name: String, repo_path: &str, excluded: &[&str]) -> Result<Self, SourceCodeError> {
        let source_files: Vec<SourceFileInfo> =
            Self::get_source_file_info_for_repo(&[repo_path], excluded)?;
        let predominant_language = Some(Self::get_predominant_language(&source_files));

        let mut statistics = Statistics::new();
        statistics.loc = Self::get_total_lines_of_code(&source_files);
        statistics.num_files = source_files.len() as i32;
        statistics.num_commits = Self::get_total_commits(repo_path).unwrap_or_default();
        statistics.size = Self::get_total_size(&source_files);

        let contributors: Vec<Contributor> = Contributor::get_git_contributors(repo_path);

        Ok(Self {
            name,
            predominant_language,
            statistics,
            contributors,
            source_files,
        })
    }
    /// Gets the [`RepositoryInfo`] as a JSON string
    pub fn get_as_json(&self) -> Result<String, SourceCodeError> {
        serde_json::to_string(&self).map_err(|err| SourceCodeError::SerializationError(err.into()))
    }
    /// Builds up the [`SourceFileInfo`]s for the repository
    fn get_source_file_info_for_repo(
        paths: &[&str],
        excluded: &[&str],
    ) -> Result<Vec<SourceFileInfo>, SourceCodeError> {
        let languages = Self::get_tokei_stats_for_repo(paths, excluded);

        let mut source_file_infos: Vec<SourceFileInfo> = Vec::new();

        for (language_name, language) in languages.iter() {
            let lang_type: LanguageType = LanguageType::new_from(language_name.to_owned());
            for file_report in &language.reports {
                let source_file_info = SourceFileInfo::get_source_file_info(
                    paths.first().unwrap(),
                    file_report,
                    &lang_type,
                )?;

                source_file_infos.push(source_file_info);
            }
        }

        Ok(source_file_infos)
    }
    /// Gets `tokei` statistics for the repository
    fn get_tokei_stats_for_repo(paths: &[&str], excluded: &[&str]) -> Languages {
        let config = Config::default();

        // Get the [`tokei::Languages`] for the repository (via 'paths')
        let mut languages = Languages::new();
        languages.get_statistics(paths, excluded, &config);

        languages
    }
    /// Gets the total size of the repository from the Vec of [`SourceFileInfo`]s
    fn get_total_size(source_file_infos: &[SourceFileInfo]) -> i64 {
        source_file_infos
            .iter()
            .map(|sfi| sfi.statistics.size)
            .sum()
    }
    /// Gets the total number of lines of code for the repository from the Vec of [`SourceFileInfo`]s
    fn get_total_lines_of_code(source_file_infos: &[SourceFileInfo]) -> i64 {
        source_file_infos.iter().map(|sfi| sfi.statistics.loc).sum()
    }
    /// Gets the predominant [`LanguageType`] for the repository from the Vec of [`SourceFileInfo`]s
    /// #Arguments:
    /// * `source_file_infos` - The Vec of [`SourceFileInfo`]s
    /// #Returns:
    /// * The [`LanguageType`] of the predominant language
    fn get_predominant_language(source_file_infos: &[SourceFileInfo]) -> LanguageType {
        let mut languages: Vec<LanguageType> = Vec::new();
        for source_file_info in source_file_infos {
            if let Some(language) = &source_file_info.language {
                languages.push(language.clone());
            }
        }
        LanguageType::get_predominant_language(&languages)
    }
    /// Gets the total number of commits for a git repository
    fn get_total_commits(repo_path: &str) -> Result<i32, SourceCodeError> {
        let repo: Repository = Repository::open(repo_path)?;
        let mut revwalk: Revwalk<'_> = repo.revwalk()?;
        revwalk.push_head()?;

        let mut total_commits: i32 = 0;

        for commit_id in revwalk {
            let _: Commit<'_> = repo.find_commit(commit_id?)?;
            total_commits += 1;
        }
        Ok(total_commits)
    }
}
/// Struct to hold the data on a repository's contributors
///
/// # Fields:
/// * `name` - The name of the contributor
/// * `last_contribution` - The date and time of the last contribution made by the contributor
/// * `percentage_contribution` - The percentage of the total contributions made by the contributor
/// * `statistics` - The [`Statistics`] on the contributor's contributions
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Contributor {
    name: String,
    last_contribution: DateTime<Utc>,
    percentage_contribution: f32,
    statistics: Statistics,
}
impl Contributor {
    pub fn new(
        name: String,
        last_contribution: DateTime<Utc>,
        percentage_contribution: f32,
        statistics: Statistics,
    ) -> Self {
        Self {
            name,
            last_contribution,
            percentage_contribution,
            statistics,
        }
    }
    /// Gets the contributors from the repository passed as the 'repo_path'.
    /// TODO: add other contributor statistics, e.g., frequency, lines of code changed in commits(?), num_files changed in commits(?), etc.
    ///
    /// #Arguments:
    /// * `repo_path` - The path to the repository
    ///
    /// #Returns:
    /// * A [`Vec`] of [`Contributor`]s
    pub fn get_git_contributors(repo_path: &str) -> Vec<Contributor> {
        let repo = Repository::open(repo_path).expect("Failed to open repository");
        let mut revwalk = repo.revwalk().expect("Failed to get revwalk");
        revwalk.push_head().expect("Failed to push head");

        let mut contributions = HashMap::<String, (DateTime<Utc>, i32)>::new();
        let mut total_contributions = 0;

        for oid in revwalk {
            if let Ok(commit) = repo.find_commit(oid.expect("Invalid oid")) {
                let name = String::from(commit.author().name().unwrap_or_default());
                let time = commit.author().when();

                let naive_date_time = NaiveDateTime::from_timestamp_opt(time.seconds(), 0).unwrap();
                let date = DateTime::<Utc>::from_naive_utc_and_offset(naive_date_time, Utc);

                let entry = contributions.entry(name).or_insert((date, 0));
                entry.1 += 1; // Increment contribution count
                if date > entry.0 {
                    entry.0 = date; // Update last contribution date if newer
                }
                total_contributions += 1;
            }
        }
        contributions
            .into_iter()
            .map(|(name, (last_contribution, num_commits))| {
                let percentage = num_commits as f32 / total_contributions as f32 * 100.0;
                let statistics = Statistics {
                    size: 0, // Not relevant for contributors
                    loc: 0,
                    num_files: 0,
                    num_commits,
                    frequency: 0.0,
                };
                Contributor::new(name, last_contribution, percentage, statistics)
            })
            .collect()
    }
}
