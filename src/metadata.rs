use html_escape::encode_safe;
use lazy_static::lazy_static;
use log::{error, info, warn};
use mdbook::{
    book::{Book, BookItem},
    errors::Error as MdBookError,
    preprocess::{Preprocessor, PreprocessorContext},
};
use regex::Regex;
use std::collections::HashMap;
use std::fmt;

use crate::cli::NAME;

lazy_static! {
    static ref METADATA_BLOCK_RE: Regex = Regex::new(r"(?s)---(.*?)---").unwrap();
    static ref METADATA_LINE_RE: Regex = Regex::new(r"^(.+?):\s*(.+)$").unwrap();
}

#[derive(Debug)]
enum MetadataError {
    ImproperlyFormattedLine(String),
}

fn parse_metadata(
    content: &str,
    continue_on_error: bool,
) -> Result<(HashMap<String, String>, String), MetadataError> {
    let mut metadata = HashMap::new();
    let content_without_metadata = METADATA_BLOCK_RE
        .replace(content, "")
        .to_string()
        .trim_start()
        .to_string(); // Remove the metadata block and trim leading whitespaces/newlines

    if let Some(caps) = METADATA_BLOCK_RE.captures(content) {
        let metadata_block = caps.get(1).unwrap().as_str();

        for line in metadata_block.lines() {
            if line.trim().is_empty() {
                continue; // Skip empty lines
            }
            match METADATA_LINE_RE.captures(line) {
                Some(caps) => {
                    let key = caps.get(1).unwrap().as_str().trim().to_string();
                    let value = caps.get(2).unwrap().as_str().trim().to_string();
                    info!("Parsed metadata: {}: {}", key, value);
                    metadata.insert(key, value);
                }
                None => {
                    if continue_on_error {
                        // Warn and continue to the next line
                        warn!("Improperly formatted metadata line skipped: '{}'", line);
                        continue;
                    } else {
                        // Return an error and halt processing
                        return Err(MetadataError::ImproperlyFormattedLine(line.to_string()));
                    }
                }
            }
        }
    }
    info!("Parsed metadata: {:?}", metadata);
    Ok((metadata, content_without_metadata.to_string()))
}

fn metadata_to_html(metadata: &HashMap<String, String>) -> String {
    let mut html_tags = String::new();
    for (key, value) in metadata {
        let escaped_value = encode_safe(&value);
        match key.as_str() {
            "title" => html_tags.push_str(&format!("<title>{}</title>\n", escaped_value)),
            _ => html_tags.push_str(&format!(
                "<meta name=\"{}\" content=\"{}\">\n",
                key, escaped_value
            )),
        }
    }
    info!("Generated HTML tags: {}", html_tags);
    html_tags
}

pub struct Metadata {
    valid_tags: Option<Vec<String>>, // Optional list of valid tags specified in the configuration
    continue_on_error: bool,         // Optional flag to continue processing after an error occurs
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

        let continue_on_error: bool = ctx
            .config
            .get_preprocessor("metadata")
            .and_then(|p| p.get("continue-on-error").cloned())
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        Self {
            valid_tags,
            continue_on_error,
        }
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
                match parse_metadata(&chap.content, self.continue_on_error) {
                    Ok((mut parsed_metadata, modified_content)) => {
                        if let Some(ref valid_tags) = self.valid_tags {
                            parsed_metadata.retain(|k, _| valid_tags.contains(k));
                        }

                        if !parsed_metadata.is_empty() {
                            let html_tags = metadata_to_html(&parsed_metadata);
                            chap.content = format!("{}\n{}", html_tags, modified_content);
                        } else {
                            chap.content = modified_content;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_parse_metadata_without_metadata_block() {
        let content = "This is a test chapter content without metadata.";

        let (metadata, content_without_metadata) = parse_metadata(content, false).unwrap();

        assert!(metadata.is_empty(), "Expected metadata to be empty");
        assert_eq!(
            content_without_metadata, content,
            "Content should remain unchanged"
        );
    }

    #[test]
    fn test_parse_metadata_with_valid_metadata_block() {
        let content_with_metadata = r#"---
title: Test Chapter
keywords: rust, testing, mdbook
released: true
---

This is the chapter content."#;

        let (metadata, content_without_metadata) =
            parse_metadata(content_with_metadata, false).unwrap();

        assert_eq!(
            metadata.get("title"),
            Some(&"Test Chapter".to_string()),
            "Title should be 'Test Chapter'"
        );
        assert_eq!(
            metadata.get("keywords"),
            Some(&"rust, testing, mdbook".to_string()),
            "Keywords should be 'rust, testing, mdbook'"
        );
        assert_eq!(
            metadata.get("released"),
            Some(&"true".to_string()),
            "Released should be 'true'"
        );

        let expected_content = "This is the chapter content.";
        assert_eq!(
            content_without_metadata, expected_content,
            "Content should not include metadata block"
        );
    }

    #[test]
    fn test_parse_metadata_with_bad_indentation() {
        let content_with_badly_indented_metadata = r#"---
    title: Test Chapter
   keywords:   rust, testing, mdbook
    released :true
---

This is the chapter content."#;

        let (metadata, content_without_metadata) =
            parse_metadata(content_with_badly_indented_metadata, false).unwrap();

        assert_eq!(
            metadata.get("title"),
            Some(&"Test Chapter".to_string()),
            "Title should be 'Test Chapter'"
        );
        assert_eq!(
            metadata.get("keywords"),
            Some(&"rust, testing, mdbook".to_string()),
            "Keywords should be 'rust, testing, mdbook'"
        );
        assert_eq!(
            metadata.get("released"),
            Some(&"true".to_string()),
            "Released should be 'true'"
        );

        let expected_content = "This is the chapter content.";
        assert_eq!(
            content_without_metadata, expected_content,
            "Content should not include metadata block"
        );
    }

    #[test]
    fn test_parse_metadata_with_bad_metadata_block() {
        let content_with_bad_metadata = r#"---
title = Incorrect Format
keywords = rust, testing, mdbook
released = false
---

This is the chapter content."#;

        let result = parse_metadata(content_with_bad_metadata, false);

        assert!(
            result.is_err(),
            "Expected an error due to bad metadata block format"
        );

        if let Err(MetadataError::ImproperlyFormattedLine(line)) = result {
            assert_eq!(
                line, "title = Incorrect Format",
                "Expected error for the improperly formatted line"
            );
        } else {
            panic!("Expected an ImproperlyFormattedLine error");
        }
    }

    #[test]
    fn test_parse_metadata_with_duplicate_keys() {
        let content_with_duplicate_keys = r#"---
title: First Title
keywords: first, set
title: Second Title
keywords: second, set
---

Chapter content."#;

        let (metadata, content_without_metadata) =
            parse_metadata(content_with_duplicate_keys, true).unwrap();

        assert_eq!(
            content_without_metadata, "Chapter content.",
            "The content should exclude the metadata block."
        );

        assert_eq!(
            metadata.get("title"),
            Some(&"Second Title".to_string()),
            "The 'title' key should reflect the last occurrence."
        );
        assert_eq!(
            metadata.get("keywords"),
            Some(&"second, set".to_string()),
            "The 'keywords' key should reflect the last occurrence."
        );

        assert_eq!(
            metadata.len(),
            2,
            "The metadata HashMap should only contain two entries, one for each unique key."
        );
    }

    #[test]
    fn test_metadata_to_html_basic() {
        let mut metadata = HashMap::new();
        metadata.insert("title".to_string(), "Example Title".to_string());
        metadata.insert("keywords".to_string(), "rust, mdbook, testing".to_string());
        metadata.insert("author".to_string(), "John Doe".to_string());

        let html_output = metadata_to_html(&metadata);

        assert!(
            html_output.contains("<title>Example Title</title>"),
            "Output should contain title tag."
        );
        assert!(
            html_output.contains("<meta name=\"keywords\" content=\"rust, mdbook, testing\">"),
            "Output should contain keywords meta tag."
        );
        assert!(
            html_output.contains("<meta name=\"author\" content=\"John Doe\">"),
            "Output should contain author meta tag."
        );
    }

    #[test]
    fn test_metadata_to_html_empty() {
        let metadata = HashMap::new();

        let html_output = metadata_to_html(&metadata);

        assert!(
            html_output.is_empty(),
            "HTML output should be empty for empty metadata."
        );
    }

    #[test]
    fn test_metadata_to_html_complex() {
        let metadata = HashMap::from([
            (
                "title".to_string(),
                "Complex & <Special> 'Characters'".to_string(),
            ),
            (
                "description".to_string(),
                r#"Testing "quotes" and other <html> elements"#.to_string(),
            ),
            (
                "keywords".to_string(),
                r#"rust,mdbook,"special, characters",<html>"#.to_string(),
            ),
        ]);

        let html_output = metadata_to_html(&metadata);

        let expected_outputs = [
            r#"<title>Complex &amp; &lt;Special&gt; &#x27;Characters&#x27;</title>"#,
            r#"<meta name="description" content="Testing &quot;quotes&quot; and other &lt;html&gt; elements">"#,
            r#"<meta name="keywords" content="rust,mdbook,&quot;special, characters&quot;,&lt;html&gt;">"#,
        ];

        // Split the output by newline and collect into a set for order-independent comparison
        let html_output_set: HashSet<_> = html_output.lines().collect();
        let expected_output_set: HashSet<_> = expected_outputs.iter().cloned().collect();

        assert_eq!(
        html_output_set, expected_output_set,
        "The HTML output should correctly handle special characters and HTML content, order-independent."
    );
    }

    #[test]
    fn test_metadata_to_html_xss_prevention() {
        let metadata = HashMap::from([
            ("title".to_string(), "Safe Title".to_string()),
            (
                "script_injection".to_string(),
                "<script>alert('XSS');</script>".to_string(),
            ),
        ]);

        let html_output = metadata_to_html(&metadata);

        // Expected outputs should escape the <, >, and other special HTML characters
        let expected_outputs = [
            r#"<title>Safe Title</title>"#,
            // Include the encoded forward slash in the expected output
            r#"<meta name="script_injection" content="&lt;script&gt;alert(&#x27;XSS&#x27;);&lt;&#x2F;script&gt;">"#,
        ];

        let html_output_set: HashSet<_> = html_output.lines().collect();
        let expected_output_set: HashSet<_> = expected_outputs.iter().cloned().collect();

        assert_eq!(
            html_output_set, expected_output_set,
            "The HTML output should escape potential XSS injection attempts, ensuring code safety."
        );
    }

    #[test]
    fn test_metadata_to_html_malicious_code() {
        let metadata = HashMap::from([
            ("title".to_string(), "Normal Title".to_string()),
            // Attempted JavaScript injection
            (
                "description".to_string(),
                r#"<script>alert("malicious code");</script>"#.to_string(),
            ),
        ]);

        let html_output = metadata_to_html(&metadata);

        let expected_outputs = [
            r#"<title>Normal Title</title>"#,
            r#"<meta name="description" content="&lt;script&gt;alert(&quot;malicious code&quot;);&lt;&#x2F;script&gt;">"#,
        ];

        let html_output_set: HashSet<_> = html_output.lines().collect();
        let expected_output_set: HashSet<_> = expected_outputs.iter().cloned().collect();

        assert_eq!(
            html_output_set, expected_output_set,
            "The HTML output should escape potentially malicious code."
        );
    }

    #[test]
    fn test_metadata_to_html_complex_structures() {
        use std::collections::BTreeMap;

        let mut nested_map = BTreeMap::new();
        nested_map.insert("nested_key", vec!["value1", "value2"]);

        let metadata = HashMap::from([
            ("title".to_string(), "Complex Structures".to_string()),
            ("complex".to_string(), format!("{:?}", nested_map)),
        ]);

        let html_output = metadata_to_html(&metadata);

        let expected_html_tags = vec![
        "<title>Complex Structures</title>",
        r#"<meta name="complex" content="{&quot;nested_key&quot;: [&quot;value1&quot;, &quot;value2&quot;]}">"#,
    ].into_iter().map(String::from).collect::<HashSet<_>>();

        let output_html_tags = html_output
            .lines()
            .map(String::from)
            .collect::<HashSet<_>>();

        assert_eq!(
            output_html_tags, expected_html_tags,
            "The HTML output should correctly handle and escape complex structures."
        );
    }

    #[test]
    fn test_metadata_to_html_large_volume() {
        let mut metadata = HashMap::new();
        for i in 0..1000 {
            metadata.insert(format!("key_{}", i), format!("value_{}", i));
        }

        let html_output = metadata_to_html(&metadata);

        for i in 0..1000 {
            let expected_key = format!("key_{}", i);
            let expected_value = format!("value_{}", i);
            let expected_tag = format!(
                "<meta name=\"{}\" content=\"{}\">",
                expected_key, expected_value
            );

            assert!(
                html_output.contains(&expected_tag),
                "The HTML output should contain the metadata entry for key {}: {}",
                expected_key,
                expected_value
            );
        }

        assert!(
            !html_output.contains("<title>"),
            "The HTML output should not contain a title tag when not specified in the metadata."
        );
    }
}
