use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDocument {
    pub title: Option<String>,
    pub frontmatter: Option<serde_json::Value>,
    pub chunks: Vec<Chunk>,
    pub tags: Vec<String>,
    pub content_hash: String,
    pub wiki_links: Vec<WikiLink>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiLink {
    pub target: String,
    pub display: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub index: usize,
    pub content: String,
    pub heading_path: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
}

/// Parse a markdown file: extract frontmatter, split into chunks
pub fn parse_markdown(content: &str, chunk_size: usize, chunk_overlap: usize) -> ParsedDocument {
    let content_hash = compute_hash(content);

    // 1. Extract frontmatter
    let (frontmatter, body, mut tags) = extract_frontmatter(content);

    // 2. Extract title: H1 first, then frontmatter `title` field as fallback
    let title = extract_title(&body).or_else(|| {
        frontmatter.as_ref()
            .and_then(|fm| fm.get("title"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    });

    // 3. Split into sections by headings
    let sections = split_by_headings(&body);

    // 4. Chunk sections that exceed chunk_size
    let chunks = chunk_sections(sections, chunk_size, chunk_overlap);

    // 5. Extract wiki links from body (using pulldown-cmark AST to skip code blocks)
    let wiki_links = extract_wiki_links(&body);

    // 6. Extract inline tags from body and merge with frontmatter tags
    let inline_tags = extract_inline_tags(&body);
    for t in inline_tags {
        if !tags.contains(&t) {
            tags.push(t);
        }
    }

    // 7. Extract aliases from frontmatter
    let aliases = frontmatter.as_ref()
        .map(|fm| extract_aliases(fm))
        .unwrap_or_default();

    ParsedDocument {
        title,
        frontmatter,
        chunks,
        tags,
        content_hash,
        wiki_links,
        aliases,
    }
}

/// Compute SHA-256 hash of content
pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Extract YAML frontmatter from markdown content
fn extract_frontmatter(content: &str) -> (Option<serde_json::Value>, String, Vec<String>) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (None, content.to_string(), vec![]);
    }

    let after_first = &trimmed[3..];
    if let Some(end_pos) = after_first.find("\n---") {
        let yaml_str = &after_first[..end_pos].trim();
        let body = &after_first[end_pos + 4..];

        match serde_yaml::from_str::<serde_yaml::Value>(yaml_str) {
            Ok(yaml_val) => {
                let json_val = serde_json::to_value(&yaml_val).ok();

                // Extract tags from frontmatter
                let tags = extract_tags_from_frontmatter(&yaml_val);

                (json_val, body.to_string(), tags)
            }
            Err(_) => (None, content.to_string(), vec![]),
        }
    } else {
        (None, content.to_string(), vec![])
    }
}

/// Extract tags from YAML frontmatter
fn extract_tags_from_frontmatter(val: &serde_yaml::Value) -> Vec<String> {
    if let serde_yaml::Value::Mapping(map) = val {
        if let Some(tags_val) = map.get(&serde_yaml::Value::String("tags".to_string())) {
            match tags_val {
                serde_yaml::Value::Sequence(seq) => {
                    return seq.iter().filter_map(|v| {
                        if let serde_yaml::Value::String(s) = v { Some(s.clone()) } else { None }
                    }).collect();
                }
                serde_yaml::Value::String(s) => {
                    return s.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
                }
                _ => {}
            }
        }
    }
    vec![]
}

/// Extract the first H1 heading as title
fn extract_title(content: &str) -> Option<String> {
    let parser = Parser::new(content);
    let mut in_h1 = false;
    let mut title = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level: HeadingLevel::H1, .. }) => {
                in_h1 = true;
            }
            Event::Text(text) if in_h1 => {
                title.push_str(&text);
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1)) => {
                if !title.is_empty() {
                    return Some(title);
                }
                in_h1 = false;
            }
            _ => {}
        }
    }
    None
}

#[derive(Debug)]
struct Section {
    heading_path: Option<String>,
    content: String,
    start_line: usize,
    end_line: usize,
}

/// Split content by headings into sections
fn split_by_headings(content: &str) -> Vec<Section> {
    let lines: Vec<&str> = content.lines().collect();
    let mut sections: Vec<Section> = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut heading_stack: Vec<(u8, String)> = Vec::new();
    let mut current_content = String::new();
    let mut section_start = 0;

    for (i, line) in lines.iter().enumerate() {
        if let Some((level, text)) = parse_heading_line(line) {
            // Save previous section
            if !current_content.trim().is_empty() {
                sections.push(Section {
                    heading_path: current_heading.clone(),
                    content: current_content.trim().to_string(),
                    start_line: section_start,
                    end_line: i.saturating_sub(1),
                });
            }

            // Update heading stack
            while heading_stack.last().map_or(false, |(l, _)| *l >= level) {
                heading_stack.pop();
            }
            heading_stack.push((level, text.to_string()));

            current_heading = Some(
                heading_stack.iter()
                    .map(|(_, h)| h.as_str())
                    .collect::<Vec<_>>()
                    .join(" > ")
            );
            current_content = String::new();
            section_start = i + 1;
        } else {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line);
        }
    }

    // Last section
    if !current_content.trim().is_empty() {
        sections.push(Section {
            heading_path: current_heading,
            content: current_content.trim().to_string(),
            start_line: section_start,
            end_line: lines.len().saturating_sub(1),
        });
    }

    sections
}

/// Parse a line as a markdown heading (returns level and text)
fn parse_heading_line(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim_start();
    let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
    if hashes >= 1 && hashes <= 6 {
        let rest = &trimmed[hashes..];
        if rest.starts_with(' ') || rest.is_empty() {
            return Some((hashes as u8, rest.trim()));
        }
    }
    None
}

/// Chunk sections into pieces of approximately chunk_size characters
fn chunk_sections(sections: Vec<Section>, chunk_size: usize, chunk_overlap: usize) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut chunk_index = 0;

    for section in sections {
        let text = &section.content;
        if text.chars().count() <= chunk_size {
            chunks.push(Chunk {
                index: chunk_index,
                content: text.to_string(),
                heading_path: section.heading_path,
                start_line: section.start_line,
                end_line: section.end_line,
            });
            chunk_index += 1;
        } else {
            // Split by sentence boundaries
            let sentences = split_sentences(text);
            let mut current = String::new();
            let mut prev_tail = String::new();

            for sentence in &sentences {
                if current.chars().count() + sentence.chars().count() > chunk_size && !current.is_empty() {
                    chunks.push(Chunk {
                        index: chunk_index,
                        content: format!("{}{}", prev_tail, current).trim().to_string(),
                        heading_path: section.heading_path.clone(),
                        start_line: section.start_line,
                        end_line: section.end_line,
                    });
                    chunk_index += 1;

                    // Overlap: keep the tail of current chunk (char-safe for multibyte)
                    let chars: Vec<char> = current.chars().collect();
                    prev_tail = if chars.len() > chunk_overlap {
                        chars[chars.len() - chunk_overlap..].iter().collect()
                    } else {
                        current.clone()
                    };
                    current = String::new();
                }
                current.push_str(sentence);
            }

            if !current.trim().is_empty() {
                chunks.push(Chunk {
                    index: chunk_index,
                    content: format!("{}{}", prev_tail, current).trim().to_string(),
                    heading_path: section.heading_path.clone(),
                    start_line: section.start_line,
                    end_line: section.end_line,
                });
                chunk_index += 1;
            }
        }
    }

    chunks
}

/// Simple sentence splitter
fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?' | '\n') {
            sentences.push(current.clone());
            current.clear();
        }
    }
    if !current.is_empty() {
        sentences.push(current);
    }
    sentences
}

/// Remove code blocks from markdown body (fenced ``` and indented)
fn strip_code_blocks(body: &str) -> String {
    let mut result = String::new();
    let mut in_fence = false;
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            result.push('\n');
            continue;
        }
        if in_fence {
            result.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

/// Extract [[wiki links]] from markdown body, skipping code blocks
fn extract_wiki_links(body: &str) -> Vec<WikiLink> {
    use std::sync::LazyLock;
    static RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\[\[([^\]]+)\]\]").unwrap());
    let clean = strip_code_blocks(body);
    let re = &*RE;
    let mut links = Vec::new();

    for cap in re.captures_iter(&clean) {
        let inner = cap[1].to_string();
        if let Some((target, display)) = inner.split_once('|') {
            links.push(WikiLink {
                target: target.trim().to_string(),
                display: Some(display.trim().to_string()),
            });
        } else {
            links.push(WikiLink {
                target: inner.trim().to_string(),
                display: None,
            });
        }
    }
    links
}

/// Extract #inline_tags from markdown body, skipping code blocks
fn extract_inline_tags(body: &str) -> Vec<String> {
    use std::sync::LazyLock;
    static RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?:^|\s)#([a-zA-Z가-힣\u{3131}-\u{318E}][a-zA-Z0-9가-힣\u{3131}-\u{318E}_/-]*)").unwrap());
    let clean = strip_code_blocks(body);
    let re = &*RE;
    let mut tags = Vec::new();

    for cap in re.captures_iter(&clean) {
        let tag = cap[1].to_string();
        if !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    tags
}

/// Extract aliases from frontmatter value
fn extract_aliases(fm: &serde_json::Value) -> Vec<String> {
    let mut aliases = Vec::new();
    if let Some(val) = fm.get("aliases") {
        match val {
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        aliases.push(s.to_string());
                    }
                }
            }
            serde_json::Value::String(s) => {
                for a in s.split(',') {
                    let trimmed = a.trim();
                    if !trimmed.is_empty() {
                        aliases.push(trimmed.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    aliases
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_frontmatter() {
        let content = "---\ntitle: Test\ntags:\n  - rust\n  - obsidian\n---\n\n# Hello\n\nBody text.";
        let (fm, body, tags) = extract_frontmatter(content);
        assert!(fm.is_some());
        assert_eq!(tags, vec!["rust", "obsidian"]);
        assert!(body.contains("Hello"));
    }

    #[test]
    fn test_no_frontmatter() {
        let content = "# Hello\n\nJust a normal document.";
        let (fm, body, tags) = extract_frontmatter(content);
        assert!(fm.is_none());
        assert!(tags.is_empty());
        assert_eq!(body, content);
    }

    #[test]
    fn test_parse_markdown_basic() {
        let content = "---\ntitle: My Note\ntags:\n  - test\n---\n\n# My Note\n\n## Section 1\n\nSome content here.\n\n## Section 2\n\nMore content.";
        let parsed = parse_markdown(content, 512, 50);

        assert_eq!(parsed.title, Some("My Note".to_string()));
        assert_eq!(parsed.tags, vec!["test"]);
        assert!(parsed.chunks.len() >= 2);
        assert!(!parsed.content_hash.is_empty());
    }

    #[test]
    fn test_heading_path() {
        let content = "## Chapter 1\n\nIntro\n\n### Section 1.1\n\nDetails here.";
        let sections = split_by_headings(content);

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].heading_path, Some("Chapter 1".to_string()));
        assert_eq!(sections[1].heading_path, Some("Chapter 1 > Section 1.1".to_string()));
    }

    #[test]
    fn test_chunking_large_section() {
        let long_text = "## Big Section\n\n".to_string() + &"This is a sentence. ".repeat(100);
        let parsed = parse_markdown(&long_text, 200, 20);
        assert!(parsed.chunks.len() > 1);
    }

    #[test]
    fn test_compute_hash() {
        let hash1 = compute_hash("hello");
        let hash2 = compute_hash("hello");
        let hash3 = compute_hash("world");
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_extract_wiki_links() {
        let body = "See [[note1]] and [[folder/note2|My Note]] for details.";
        let links = extract_wiki_links(body);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, "note1");
        assert!(links[0].display.is_none());
        assert_eq!(links[1].target, "folder/note2");
        assert_eq!(links[1].display.as_deref(), Some("My Note"));
    }

    #[test]
    fn test_wiki_links_skip_code_blocks() {
        let body = "Normal [[link]]\n\n```\n[[not-a-link]]\n```\n\nAnother [[real-link]].";
        let links = extract_wiki_links(body);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, "link");
        assert_eq!(links[1].target, "real-link");
    }

    #[test]
    fn test_extract_inline_tags() {
        let body = "This is #rust and #한국어 content.\n\n```\n#not-a-tag\n```";
        let tags = extract_inline_tags(body);
        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"한국어".to_string()));
        assert!(!tags.contains(&"not-a-tag".to_string()));
    }

    #[test]
    fn test_extract_aliases() {
        let fm: serde_json::Value = serde_json::json!({
            "title": "Test",
            "aliases": ["AI", "인공지능", "Artificial Intelligence"]
        });
        let aliases = extract_aliases(&fm);
        assert_eq!(aliases.len(), 3);
        assert_eq!(aliases[0], "AI");
        assert_eq!(aliases[1], "인공지능");
    }

    #[test]
    fn test_extract_aliases_string() {
        let fm: serde_json::Value = serde_json::json!({
            "aliases": "AI, 인공지능"
        });
        let aliases = extract_aliases(&fm);
        assert_eq!(aliases.len(), 2);
    }

    #[test]
    fn test_parse_markdown_with_links_and_tags() {
        let content = "---\ntitle: Test\ntags:\n  - frontmatter-tag\naliases:\n  - MyAlias\n---\n\n# Test\n\nSee [[other-note]] and #inline-tag here.";
        let parsed = parse_markdown(content, 512, 50);
        assert_eq!(parsed.wiki_links.len(), 1);
        assert_eq!(parsed.wiki_links[0].target, "other-note");
        assert!(parsed.tags.contains(&"frontmatter-tag".to_string()));
        assert!(parsed.tags.contains(&"inline-tag".to_string()));
        assert_eq!(parsed.aliases, vec!["MyAlias"]);
    }
}
