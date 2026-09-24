use colander::semver::*;

#[test]
fn parses_three_plain_integers() {
    assert_eq!(
        parse("1.2.3").unwrap(),
        VersionParts {
            major: 1,
            minor: 2,
            patch: 3
        }
    );
    assert_eq!(parse("0.0.0").unwrap().major, 0);
    assert!(parse("1.0").is_err());
    assert!(parse("1.0.0.0").is_err());
    assert!(parse("abc").is_err());
    assert!(parse("").is_err());
    assert!(parse("1.-1.0").is_err());
    // Strict: no whitespace, no blank segments, no leading zeros, no sign.
    assert!(parse(" 1 . 2 . 3 ").is_err());
    assert!(parse("1..0.0").is_err());
    assert!(parse("01.0.0").is_err());
    assert!(parse("-0.0.0").is_err());
    assert!(parse("+1.0.0").is_err());
    // Pre-release and build metadata are not permitted on a component version.
    assert!(parse("1.0.0-beta").is_err());
    assert!(parse("1.0.0+build").is_err());
    assert!(parse("v1.0.0").is_err());
}

#[test]
fn rejects_invalid_published_entries() {
    assert!(next_version(&["1.0"], Bump::Patch).is_err());
    assert!(next_version(&[""], Bump::Patch).is_err());
    assert!(next_version(&["1.0.0", "abc"], Bump::Patch).is_err());
    assert!(next_version(&["01.0.0"], Bump::Patch).is_err());
}

#[test]
fn bumps_the_highest_version_with_the_given_bump() {
    assert_eq!(next_version::<&str>(&[], Bump::Patch).unwrap(), "1.0.0");
    assert_eq!(next_version(&["1.0.0"], Bump::Patch).unwrap(), "1.0.1");
    assert_eq!(
        next_version(&["1.2.3", "1.10.0", "1.9.9"], Bump::Patch).unwrap(),
        "1.10.1"
    );
    assert_eq!(
        next_version(&["1.2.3", "1.10.0"], Bump::Minor).unwrap(),
        "1.11.0"
    );
    assert_eq!(
        next_version(&["1.2.3", "2.0.0"], Bump::Major).unwrap(),
        "3.0.0"
    );
}

#[test]
fn rejects_an_unknown_bump_name() {
    assert!(Bump::parse("banana").is_err());
    assert!(Bump::parse("PATCH").is_err());
    assert_eq!(Bump::parse("patch").unwrap(), Bump::Patch);
}
