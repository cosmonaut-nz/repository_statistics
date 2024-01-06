use fastembed::{EmbeddingBase, EmbeddingModel, FlagEmbedding, InitOptions};
use qdrant_client::client::QdrantClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{errors::SourceCodeError, repository::RepositoryInfo, source::SourceFileInfo};

/// Creates an embedding from the repository data, then stores it in a vector database
/// For each source file represented as:
/// {
///     "source_file": "path/to/source/file",
///     "data": {
///         "language": "name",
///         "id_hash": "SHA256 hash of the file contents",
///         "contents": "Source file contents",
///         "size_sentiment": 123123,
///         "loc_sentiment": 124124,
///         "frequency_sentiment": 124124
///     }
/// }

#[derive(Serialize, Deserialize)]
pub struct FileToEmbed {
    pub name: String,
    pub data: FileData,
}
#[derive(Serialize, Deserialize)]
pub struct FileData {
    pub language: String,
    pub id_hash: String,
    pub contents: String,
    pub size_sentiment: f32,
    pub loc_sentiment: f32,
    pub frequency_sentiment: f32,
}

///
pub async fn create_repository_embedding(stats: RepositoryInfo) -> Result<(), SourceCodeError> {
    log::info!("starting embedding");
    let model: FlagEmbedding = FlagEmbedding::try_new(InitOptions {
        model_name: EmbeddingModel::AllMiniLML6V2,
        show_download_message: true,
        ..Default::default()
    })?;

    // Get the list of source files.
    // Flatten into a Vec<SourceFile>
    // Serialize each FileEmbedding into a JSON string and add to a Vec<[String, String]>, where the key is the filename and the value is the JSON string.
    // Embed the Vec<[String, String]>.
    //
    // Derive 'sentiment' from Statistics:
    //
    //    size_sentiment = -log10(size) (larger size = more negative sentiment)
    //    loc_sentiment = -log10(loc)
    //    frequency_sentiment = -log10(frequency)
    //
    //    TODO: for contributors, reverse the sentiment:
    //    contributor_frequency_sentiment = log10(frequency)
    //
    // Example:
    // pub async fn embed_repo<M: EmbeddingsModel + Send + Sync>(
    //     repository: &Repository,
    //     files: Vec<File>,
    //     model: &M,
    // ) -> Result<RepositoryEmbeddings> {
    //     let content: Vec<String> = files.par_iter().map(|file| file.content.clone()).collect();

    //     let embeddings: Vec<Embeddings> = model.embed(content)?;

    //     let file_embeddings: Vec<FileEmbeddings> = embeddings
    //         .into_par_iter()
    //         .zip(files.into_par_iter())
    //         .map(|(embeddings, file)| FileEmbeddings {
    //             path: file.path,
    //             embeddings,
    //         })
    //         .collect();

    //     Ok(RepositoryEmbeddings {
    //         repo_id: repository.to_string(),
    //         file_embeddings,
    //     })
    // }

    // let stats_json = stats.get_as_json()?;

    // log::info!("Stats JSON size: {}", stats_json.len());

    // iterate over the source files and create a [`File`] struct for each one
    let files: Vec<FileToEmbed> = stats
        .source_files
        .iter()
        .map(|source_file_info| map_source_file_info_to_file(source_file_info))
        .collect();
    // Serialize each File struct into a JSON string
    let files_json: Vec<String> = files
        .iter()
        .map(|file: &FileToEmbed| serde_json::to_string(file).unwrap())
        .collect();
    // For each JSON string, flatten it into a Vec<String> after extracting the source_file key
    let mut result: Vec<String> = Vec::new();
    for file_json in files_json {
        let json_value: Value = serde_json::from_str(&file_json).unwrap();
        let key = json_value["name"].as_str().unwrap().to_string();
        let flattened_json = flatten_json(&json_value["data"]);

        for value in flattened_json {
            let entry = format!("{}: {}", key, value);
            result.push(entry);
        }
    }

    let embeddings = model.embed(result, None)?;

    // TODO create a viable struct to hold the embeddings
    // TODO insert into the Qdrant database

    log::info!("Embeddings length: {:?}", embeddings);

    // TODO: configure the Qdrant server URL from an environment variable
    let _client = QdrantClient::from_url("http://localhost:6334").build()?;

    Ok(())
}
/// Maps a SourceFileInfo to a File struct
fn map_source_file_info_to_file(source_file_info: &SourceFileInfo) -> FileToEmbed {
    let language = source_file_info
        .language
        .as_ref()
        .map(|l| l.name.clone())
        .unwrap_or_default();
    let id_hash = source_file_info.id_hash.clone().unwrap_or_default();
    let contents = source_file_info.get_source_file_contents();

    let statistics = source_file_info.statistics.clone();
    let size_sentiment = negative_sentiment_for_int(statistics.size);
    let loc_sentiment = negative_sentiment_for_int(statistics.loc);
    let frequency_sentiment = negative_sentiment_for_float(statistics.frequency);

    FileToEmbed {
        name: source_file_info.name.clone(),
        data: FileData {
            language,
            id_hash,
            contents,
            size_sentiment,
            loc_sentiment,
            frequency_sentiment,
        },
    }
}

/// Creates a negative sentiment value from a number using - ilog10(num)
/// Used to derive sentiment from the size and loc of a source file
fn negative_sentiment_for_int(num: i64) -> f32 {
    if num == 0 {
        return 0.0;
    }
    let sentiment = num.ilog10() as f32;
    -sentiment
}
/// Creates a negative sentiment value from a number using - log10(num)
/// Used to derive sentiment from the frequency of commits to a source file
fn negative_sentiment_for_float(num: f32) -> f32 {
    if num == 0.0 {
        return 0.0;
    }
    let sentiment = num.log10();
    -sentiment
}

/// Flattens a valid JSON string into a Vec<String>
fn flatten_json(json: &Value) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut stack = vec![(json, String::new())];

    while let Some((value, path)) = stack.pop() {
        match value {
            Value::Object(obj) => {
                for (key, value) in obj {
                    let child_path = format!("{}/{}", path, key);
                    stack.push((value, child_path));
                }
            }
            Value::Array(arr) => {
                for (index, value) in arr.iter().enumerate() {
                    let child_path = format!("{}/{}", path, index);
                    stack.push((value, child_path));
                }
            }
            _ => {
                tokens.push(format!("{}/{}", path, value));
            }
        }
    }

    tokens
}
