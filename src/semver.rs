//! Minimal `major.minor.patch` version parsing and ordering.

use std::cmp::Ordering;

use crate::error::{ColanderError, Result};

const DEFAULT_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionParts {
    pub major: i32,
    pub minor: i32,
    pub patch: i32,
}

impl std::fmt::Display for VersionParts {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Ord for VersionParts {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

impl PartialOrd for VersionParts {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Exactly three dot-separated integer segments are required, each non-negative.
/// Every segment is trimmed first and blank segments are dropped before counting,
/// so extra dots do not add segments.
pub fn parse(version: &str) -> Result<VersionParts> {
    let segments: Vec<&str> = version
        .split('.')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect();

    if segments.len() == 3 {
        let major = segments[0].parse::<i32>();
        let minor = segments[1].parse::<i32>();
        let patch = segments[2].parse::<i32>();
        if let (Ok(major), Ok(minor), Ok(patch)) = (major, minor, patch)
            && major >= 0
            && minor >= 0
            && patch >= 0
        {
            return Ok(VersionParts {
                major,
                minor,
                patch,
            });
        }
    }

    Err(ColanderError::new(format!(
        "Invalid semantic version: {version}"
    )))
}

pub fn ensure_valid(version: &str) -> Result<()> {
    parse(version).map(|_| ())
}

/// Compares two versions; an invalid version string is reported as an error.
pub fn compare(left: &str, right: &str) -> Result<Ordering> {
    Ok(parse(left)?.cmp(&parse(right)?))
}

/// The highest published version with its patch bumped, or `1.0.0` when nothing
/// is published.
pub fn next_version<S: AsRef<str>>(published: &[S]) -> Result<String> {
    let mut latest: Option<VersionParts> = None;
    for candidate in published {
        let parts = parse(candidate.as_ref())?;
        // Ties keep the last maximal element, not the first.
        if latest.is_none_or(|current| parts >= current) {
            latest = Some(parts);
        }
    }

    match latest {
        None => Ok(DEFAULT_VERSION.to_string()),
        Some(parts) => Ok(VersionParts {
            patch: parts.patch + 1,
            ..parts
        }
        .to_string()),
    }
}

/// Comparator used when ordering dependency metadata by version.
///
/// Every version reaching this point has already passed [`ensure_valid`], so the
/// fallback to literal string ordering is unreachable in practice and only keeps
/// the comparator total.
pub fn version_cmp(left: &str, right: &str) -> Ordering {
    match (parse(left), parse(right)) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.cmp(right),
    }
}
