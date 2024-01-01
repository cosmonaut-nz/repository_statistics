use crate::source_data::{CodeStats, Language, Report, SourceFile};
use errors::SourceCodeError;

use tokei::{Config, Languages};

pub fn get_statistics_for_repository_as_json(
    paths: &[&str],
    excluded: &[&str],
) -> Result<String, SourceCodeError> {
    let stats = get_statistics_for_repository(paths, excluded);

    // Serialize `stats` to JSON
    let json = serde_json::to_string(&stats)
        .map_err(|err| SourceCodeError::SerializationError(Box::new(err)))?;

    Ok(json)
}

pub fn get_statistics_for_repository(paths: &[&str], excluded: &[&str]) -> Vec<Language> {
    let config = Config::default();

    let mut languages = Languages::new();
    languages.get_statistics(paths, excluded, &config);

    languages
        .iter()
        .filter(|(_, language)| language.code != 0)
        .map(|(language_type, language)| {
            Language {
                name: language_type.to_string(),
                blanks: language.blanks as u32,
                code: language.code as u32,
                comments: language.comments as u32,
                reports: language
                    .reports
                    .iter()
                    .map(|report| {
                        Report {
                            stats: CodeStats {
                                blanks: report.stats.blanks as u32,
                                code: report.stats.code as u32,
                                comments: report.stats.comments as u32,
                                blobs: report
                                    .stats
                                    .blobs
                                    .values()
                                    .map(|stats| {
                                        CodeStats {
                                            blanks: stats.blanks as u32,
                                            code: stats.code as u32,
                                            comments: stats.comments as u32,
                                            blobs: vec![], // Empty for now, adjust if needed
                                        }
                                    })
                                    .collect(),
                            },
                            source_files: vec![SourceFile {
                                name: report.name.to_string_lossy().to_string(),
                                source: None,
                            }],
                        }
                    })
                    .collect(),
                children: vec![], // Empty for now, adjust if needed
                inaccurate: language.inaccurate,
            }
        })
        .collect()
}

pub mod errors {
    use std::error::Error;
    use std::fmt;

    #[derive(Debug)]
    pub enum SourceCodeError {
        SerializationError(Box<dyn Error>),
    }

    impl fmt::Display for SourceCodeError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Source code error: {:?}", self)
        }
    }

    impl Error for SourceCodeError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                SourceCodeError::SerializationError(err) => Some(&**err),
            }
        }
    }
}

pub mod source_data {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Language {
        pub name: String,
        pub blanks: u32,
        pub code: u32,
        pub comments: u32,
        pub reports: Vec<Report>,
        pub children: Vec<Language>,
        pub inaccurate: bool,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Report {
        pub stats: CodeStats,
        pub source_files: Vec<SourceFile>,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct SourceFile {
        pub name: String,
        pub source: Option<String>,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct CodeStats {
        pub blanks: u32,
        pub code: u32,
        pub comments: u32,
        pub blobs: Vec<CodeStats>,
    }
}
