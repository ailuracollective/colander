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

/// Exactly three dot-separated numeric identifiers, under SemVer 2.0.0's rules
/// for numeric identifiers: ASCII digits only, no sign, no whitespace, no blank
/// segments, and no leading zero unless the segment is exactly "0".
/// Pre-release and build metadata are not permitted on a component version.
pub fn parse(version: &str) -> Result<VersionParts> {
    let segments: Vec<&str> = version.split('.').collect();
    if segments.len() == 3
        && segments.iter().all(|segment| is_plain_integer(segment))
        && let (Ok(major), Ok(minor), Ok(patch)) = (
            segments[0].parse::<i32>(),
            segments[1].parse::<i32>(),
            segments[2].parse::<i32>(),
        )
    {
        return Ok(VersionParts {
            major,
            minor,
            patch,
        });
    }

    Err(ColanderError::new(format!(
        "INVALID_SEMVER: invalid semantic version: {version}"
    )))
}

fn is_plain_integer(segment: &str) -> bool {
    !segment.is_empty()
        && segment.bytes().all(|byte| byte.is_ascii_digit())
        && (segment.len() == 1 || !segment.starts_with('0'))
}

pub fn ensure_valid(version: &str) -> Result<()> {
    parse(version).map(|_| ())
}

/// Compares two versions; an invalid version string is reported as an error.
pub fn compare(left: &str, right: &str) -> Result<Ordering> {
    Ok(parse(left)?.cmp(&parse(right)?))
}

/// The intended version bump. It is always supplied by the caller and never
/// inferred (SPEC P-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bump {
    Patch,
    Minor,
    Major,
}

impl Bump {
    /// Parses a bump name; anything else is an error.
    pub fn parse(name: &str) -> Result<Bump> {
        match name {
            "patch" => Ok(Bump::Patch),
            "minor" => Ok(Bump::Minor),
            "major" => Ok(Bump::Major),
            other => Err(ColanderError::new(format!(
                "INVALID_SEMVER: unknown bump '{other}' (expected 'patch', 'minor' or 'major')."
            ))),
        }
    }

    fn apply(self, parts: VersionParts) -> VersionParts {
        match self {
            Bump::Patch => VersionParts {
                patch: parts.patch + 1,
                ..parts
            },
            Bump::Minor => VersionParts {
                major: parts.major,
                minor: parts.minor + 1,
                patch: 0,
            },
            Bump::Major => VersionParts {
                major: parts.major + 1,
                minor: 0,
                patch: 0,
            },
        }
    }
}

/// The highest published version with `bump` applied, or `1.0.0` when nothing
/// is published.
pub fn next_version<S: AsRef<str>>(published: &[S], bump: Bump) -> Result<String> {
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
        Some(parts) => Ok(bump.apply(parts).to_string()),
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
