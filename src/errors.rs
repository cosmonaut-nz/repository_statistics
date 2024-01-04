use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum SourceCodeError {
    SerializationError(anyhow::Error),
    QdrantError(anyhow::Error),
    GitError(git2::Error),
    ConversionError(std::num::TryFromIntError),
    FileReadError(std::io::Error),
    FilePathError(std::path::StripPrefixError),
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
            SourceCodeError::QdrantError(err) => Some(&**err),
            SourceCodeError::GitError(err) => Some(err),
            SourceCodeError::ConversionError(err) => Some(err),
            SourceCodeError::FileReadError(err) => Some(err),
            SourceCodeError::FilePathError(err) => Some(err),
        }
    }
}
impl From<anyhow::Error> for SourceCodeError {
    fn from(err: anyhow::Error) -> Self {
        SourceCodeError::QdrantError(err)
    }
}
impl From<git2::Error> for SourceCodeError {
    fn from(error: git2::Error) -> Self {
        SourceCodeError::GitError(error)
    }
}
impl From<std::num::TryFromIntError> for SourceCodeError {
    fn from(error: std::num::TryFromIntError) -> Self {
        SourceCodeError::ConversionError(error)
    }
}
impl From<std::io::Error> for SourceCodeError {
    fn from(error: std::io::Error) -> Self {
        SourceCodeError::FileReadError(error)
    }
}
impl From<std::path::StripPrefixError> for SourceCodeError {
    fn from(error: std::path::StripPrefixError) -> Self {
        SourceCodeError::FilePathError(error)
    }
}
