//! Documentation-truth guard.
//!
//! These tests are string-based on purpose: they lock down claims in the prose
//! under `README.md`, `AGENTS.md` and `docs/` that `cargo test` would otherwise
//! never check — the export count, the pages that must be listed, the links
//! that must resolve, and the repository state the docs describe.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The fixed number of `no_mangle` exports in the shared library.
const EXPORT_COUNT: usize = 12;
/// The number of those exports the generated header declares.
const DECLARED_COUNT: usize = 10;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

fn docs_markdown_files() -> Vec<PathBuf> {
    let docs = root().join("docs");
    let mut files = Vec::new();
    for entry in fs::read_dir(&docs)
        .unwrap_or_else(|error| panic!("cannot list {}: {error}", docs.display()))
    {
        let path = entry.expect("directory entry").path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(path);
        }
    }
    files.sort();
    files
}

/// Every markdown file the guards scan: `README.md`, `AGENTS.md` and `docs/*.md`.
fn markdown_files() -> Vec<PathBuf> {
    let mut files = vec![root().join("README.md"), root().join("AGENTS.md")];
    files.extend(docs_markdown_files());
    files.sort();
    files
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(dir).unwrap_or_else(|error| panic!("cannot list {}: {error}", dir.display()))
    {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn no_mangle_count() -> usize {
    let ffi = root().join("src").join("ffi");
    let mut files = Vec::new();
    collect_rust_files(&ffi, &mut files);
    files
        .iter()
        .map(|file| read(file).matches("no_mangle").count())
        .sum()
}

/// Distinct `colander_*` names inside the fenced block after the marker.
fn readme_exported_symbols(readme: &str) -> Vec<String> {
    let marker = "exports these symbols";
    let marker_at = readme
        .find(marker)
        .unwrap_or_else(|| panic!("README.md: missing marker '{marker}'"));
    let after = &readme[marker_at + marker.len()..];
    let fence_open = after
        .find("```")
        .unwrap_or_else(|| panic!("README.md: no fenced block after '{marker}'"));
    let rest = &after[fence_open + 3..];
    let body_start = rest.find('\n').map(|offset| offset + 1).unwrap_or(0);
    let body = &rest[body_start..];
    let fence_close = body
        .find("```")
        .unwrap_or_else(|| panic!("README.md: fenced block after '{marker}' does not close"));

    let mut names = BTreeSet::new();
    for token in body[..fence_close].split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
        if token.starts_with("colander_") && token.len() > "colander_".len() {
            names.insert(token.to_string());
        }
    }
    names.into_iter().collect()
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn contains_word(haystack: &str, needle: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut from = 0;
    while let Some(offset) = haystack[from..].find(needle) {
        let start = from + offset;
        let end = start + needle.len();
        let before_ok = start == 0 || !is_word_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_word_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

/// Standalone numeric tokens on a line: digit runs inside a token that also
/// contains a letter (like `UTF-8`) are not counts and are ignored.
fn digit_numbers(line: &str) -> Vec<u64> {
    let mut numbers = Vec::new();
    let is_delimiter = |c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_');
    for token in line.split(is_delimiter) {
        if token.is_empty() || token.chars().any(|c| c.is_ascii_alphabetic()) {
            continue;
        }
        let mut current = String::new();
        for ch in token.chars() {
            if ch.is_ascii_digit() {
                current.push(ch);
            } else if !current.is_empty() {
                numbers.push(current.parse().expect("a run of digits parses"));
                current.clear();
            }
        }
        if !current.is_empty() {
            numbers.push(current.parse().expect("a run of digits parses"));
        }
    }
    numbers
}

fn line_of(content: &str, offset: usize) -> usize {
    content[..offset].matches('\n').count() + 1
}

/// Inline markdown link targets as `(line number, raw target)`.
fn link_targets(content: &str) -> Vec<(usize, String)> {
    let mut targets = Vec::new();
    let mut from = 0;
    while let Some(open) = content[from..].find("](") {
        let start = from + open + 2;
        let Some(close) = content[start..].find(')') else {
            break;
        };
        let end = start + close;
        targets.push((line_of(content, start), content[start..end].to_string()));
        from = end + 1;
    }
    targets
}

#[test]
fn export_count() {
    let symbols = no_mangle_count();
    let readme = read(&root().join("README.md"));
    let documented = readme_exported_symbols(&readme).len();
    let entry_points = read(&root().join("docs").join("entry-points.md"));
    let rows = entry_points
        .lines()
        .filter(|line| line.trim_start().starts_with("| `colander_"))
        .count();

    assert_eq!(
        symbols, EXPORT_COUNT,
        "src/ffi: expected {EXPORT_COUNT} `no_mangle` exports, found {symbols}"
    );
    assert_eq!(
        documented, symbols,
        "README.md: the 'exports these symbols' block lists {documented} names but src/ffi exports {symbols}"
    );
    assert_eq!(
        rows, symbols,
        "docs/entry-points.md: {rows} `colander_` table rows but src/ffi exports {symbols}"
    );
}

#[test]
fn no_export_count_contradiction() {
    let keywords = ["export", "exports", "exported", "symbol", "symbols"];
    let spelled = [
        ("nine", 9_u64),
        ("ten", 10),
        ("eleven", 11),
        ("twelve", 12),
        ("thirteen", 13),
    ];
    let expected = [EXPORT_COUNT as u64, DECLARED_COUNT as u64];

    for path in markdown_files() {
        let content = read(&path);
        for (index, line) in content.lines().enumerate() {
            let lower = line.to_lowercase();
            if !keywords.iter().any(|word| contains_word(&lower, word)) {
                continue;
            }
            let line_number = index + 1;
            for (word, value) in spelled {
                if contains_word(&lower, word) && !expected.contains(&value) {
                    panic!(
                        "{}:{line_number}: export/symbol line states '{word}' ({value}), expected {EXPORT_COUNT} or {DECLARED_COUNT}",
                        path.display()
                    );
                }
            }
            for number in digit_numbers(line) {
                if !expected.contains(&number) {
                    panic!(
                        "{}:{line_number}: export/symbol line states '{number}', expected {EXPORT_COUNT} or {DECLARED_COUNT}",
                        path.display()
                    );
                }
            }
        }
    }
}

#[test]
fn every_doc_page_is_listed() {
    let docs = root().join("docs");
    let readme = read(&root().join("README.md"));
    let docs_readme = read(&docs.join("README.md"));

    for path in docs_markdown_files() {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if name == "README.md" {
            continue;
        }
        assert!(
            readme.contains(name),
            "README.md does not reference docs/{name}"
        );
        assert!(
            docs_readme.contains(name),
            "docs/README.md does not reference docs/{name}"
        );
    }
}

#[test]
fn relative_links_resolve() {
    let mut files = vec![root().join("README.md")];
    files.extend(docs_markdown_files());

    for path in files {
        let content = read(&path);
        let base = path.parent().expect("file has a parent directory");
        for (line_number, raw) in link_targets(&content) {
            let target = raw
                .split_whitespace()
                .next()
                .unwrap_or("")
                .split('#')
                .next()
                .unwrap_or("");
            if target.is_empty()
                || target.starts_with("http://")
                || target.starts_with("https://")
                || target.starts_with("mailto:")
            {
                continue;
            }
            let resolved = if target.starts_with('/') {
                PathBuf::from(target)
            } else {
                base.join(target)
            };
            assert!(
                resolved.exists(),
                "{}:{line_number}: link target '{raw}' does not resolve to {}",
                path.display(),
                resolved.display()
            );
        }
    }
}

#[test]
fn no_stale_repo_state_claims() {
    // These phrases described a pre-repository state that became false once the
    // repository gained an `origin` remote, commits and the `v0.1.0` tag.
    let stale = [
        "unborn",
        "has not been exercised",
        "no commits and no remote",
    ];
    for name in ["README.md", "AGENTS.md"] {
        let path = root().join(name);
        let content = read(&path);
        let lower = content.to_lowercase();
        for phrase in stale {
            if let Some(offset) = lower.find(phrase) {
                panic!(
                    "{}:{}: stale repository-state claim '{phrase}'",
                    path.display(),
                    line_of(&lower, offset)
                );
            }
        }
    }
}
