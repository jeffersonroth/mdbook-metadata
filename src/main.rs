use clap::Command;
use env_logger;
use lazy_static::lazy_static;
use log::{error, info};
use mdbook::{
    book::{Book, BookItem},
    errors::Error as MdBookError,
    preprocess::{CmdPreprocessor, Preprocessor, PreprocessorContext},
};
use regex::Regex;
use serde_json;
use std::collections::HashMap;
use std::{fmt, io, process};

pub const NAME: &str = "metadata-preprocessor";

lazy_static! {
    // Regex to capture metadata blocks
    static ref METADATA_BLOCK_RE: Regex = Regex::new(r"(?s)---(.*?)---").unwrap();
    // Regex to parse individual metadata lines
    static ref METADATA_LINE_RE: Regex = Regex::new(r"^(.+?):\s*(.+)$").unwrap();
}

#[derive(Debug)]
enum MetadataError {
    ImproperlyFormattedLine(String),
}

fn parse_metadata(content: &str) -> Result<(HashMap<String, String>, String), MetadataError> {
    let mut metadata = HashMap::new();
    let content_without_metadata = METADATA_BLOCK_RE.replace(content, ""); // Remove the metadata block
    if let Some(caps) = METADATA_BLOCK_RE.captures(content) {
        let metadata_block = caps.get(1).unwrap().as_str();

        for line in metadata_block.lines() {
            if line.trim().is_empty() {
                continue; // Skip empty lines
            }
            let caps = METADATA_LINE_RE
                .captures(line)
                .ok_or_else(|| MetadataError::ImproperlyFormattedLine(line.to_string()))?;
            let key = caps.get(1).unwrap().as_str().trim().to_string();
            let value = caps.get(2).unwrap().as_str().trim().to_string();
            info!("Parsed metadata: {}: {}", key, value);
            metadata.insert(key, value);
        }
    }
    info!("Parsed metadata: {:?}", metadata);
    Ok((metadata, content_without_metadata.to_string())) // Always returns Ok, even if the metadata block is absent.
}

fn metadata_to_html(metadata: &HashMap<String, String>) -> String {
    let mut html_tags = String::new();
    for (key, value) in metadata {
        match key.as_str() {
            "title" => html_tags.push_str(&format!("<title>{}</title>\n", value)),
            _ => html_tags.push_str(&format!("<meta name=\"{}\" content=\"{}\">\n", key, value)),
        }
    }
    info!("Generated HTML tags: {}", html_tags);
    html_tags
}

pub struct Metadata {
    valid_tags: Option<Vec<String>>, // Optional list of valid tags specified in the configuration
}

impl Metadata {
    pub fn new(ctx: &PreprocessorContext) -> Self {
        let valid_tags: Option<Vec<String>> = ctx
            .config
            .get_preprocessor("metadata")
            .and_then(|p| p.get("valid-tags").cloned())
            .map(|tags| {
                tags.as_array()
                    .unwrap()
                    .iter()
                    .map(|t| t.as_str().unwrap().to_string())
                    .collect()
            });

        Self { valid_tags }
    }
}

impl fmt::Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MetadataError::ImproperlyFormattedLine(ref line) => {
                write!(f, "Improperly formatted metadata line: '{}'", line)
            }
        }
    }
}

impl Preprocessor for Metadata {
    fn name(&self) -> &str {
        NAME
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book, MdBookError> {
        let mut errors: Vec<String> = Vec::new();

        book.for_each_mut(|item: &mut BookItem| {
            if let BookItem::Chapter(ref mut chap) = item {
                match parse_metadata(&chap.content) {
                    Ok((mut parsed_metadata, modified_content)) => {
                        if let Some(ref valid_tags) = self.valid_tags {
                            parsed_metadata.retain(|k, _| valid_tags.contains(k));
                        }

                        // If there is metadata after filtering with valid tags, generate HTML tags and prepend
                        if !parsed_metadata.is_empty() {
                            let html_tags = metadata_to_html(&parsed_metadata);
                            chap.content = format!("{}\n{}", html_tags, modified_content);
                        // Prepend the generated HTML tags to the modified content
                        } else {
                            chap.content = modified_content; // If no metadata, just use the modified content
                        }
                    }
                    Err(e) => {
                        let error_msg =
                            format!("Error parsing metadata in chapter '{}': {}", chap.name, e);
                        errors.push(error_msg.clone());
                        error!("{}", error_msg);
                    }
                }
            }
        });

        if errors.is_empty() {
            Ok(book)
        } else {
            error!(
                "Errors occurred during preprocessing: \n{}",
                errors.join("\n")
            );
            Err(MdBookError::from(anyhow::Error::msg(errors.join("\n"))))
        }
    }
}

pub fn make_app() -> Command {
    Command::new(NAME).about("An mdbook preprocessor that parses markdown metadata")
}

fn main() {
    env_logger::init();

    if std::env::args().nth(1).as_deref() == Some("supports") {
        process::exit(0);
    }

    let _app = make_app();

    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin()).expect("Failed to parse input");

    // Use Metadata::new to initialize the preprocessor so it can set up based on the context
    let preprocessor = Metadata::new(&ctx);

    let processed_book = preprocessor
        .run(&ctx, book)
        .expect("Failed to process book");

    serde_json::to_writer(io::stdout(), &processed_book).expect("Failed to emit processed book");
}
