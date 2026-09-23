use colander::semver::*;

#[test]
fn parses_three_segments() {
    assert_eq!(
        parse("1.2.3").unwrap(),
        VersionParts {
            major: 1,
            minor: 2,
            patch: 3
        }
    );
    assert!(parse("1.0").is_err());
    assert!(parse("1.0.0.0").is_err());
    assert!(parse("abc").is_err());
    assert!(parse("").is_err());
    assert!(parse("1.-1.0").is_err());
    assert_eq!(parse(" 1 . 2 . 3 ").unwrap().major, 1);
    assert_eq!(parse("01.0.0").unwrap().major, 1);
    assert_eq!(parse("-0.0.0").unwrap().major, 0);
}

#[test]
fn rejects_invalid_published_entries() {
    assert!(next_version(&["1.0"]).is_err());
    assert!(next_version(&[""]).is_err());
    assert!(next_version(&["1.0.0", "abc"]).is_err());
}

#[test]
fn bumps_the_highest_patch() {
    assert_eq!(next_version::<&str>(&[]).unwrap(), "1.0.0");
    assert_eq!(next_version(&["1.0.0"]).unwrap(), "1.0.1");
    assert_eq!(
        next_version(&["1.2.3", "1.10.0", "1.9.9"]).unwrap(),
        "1.10.1"
    );
}
