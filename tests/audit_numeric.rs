//! Regression tests for the numeric invariants found in the fourth audit.
//!
//! Three surfaces used to disagree about numbers:
//!
//! 1. `uniqueItems` bucketed values by hash but compared them with a different
//!    equality, so `-0.0` and `0.0` could land in different buckets and both
//!    survive as "unique" even though they are the same value.
//! 2. The expression operators compared through `f64`, so `i64::MAX` matched
//!    the nearest double and two large integers collapsed into one.
//! 3. The JSON Schema subset's equality compared through `f64`, so
//!    `9007199254740993` matched `9007199254740992.0`.
//!
//! The invariant every test here defends: equality is by mathematical value,
//! and one pair of values gets one answer from every surface.

use std::cmp::Ordering;

use colander::json;
use colander::rules::{RowSet, Val, compare_values, evaluate_expression};
use colander::schema::validate_text;
use indexmap::IndexMap;

/// The operator surface and the calculated-value surface must never disagree.
#[test]
fn ordering_and_equality_agree_on_every_pair() {
    let numbers = [
        Val::Int(i64::MAX),
        Val::Int(i64::MIN),
        Val::Int(9_007_199_254_740_993),
        Val::Int(9_007_199_254_740_992),
        Val::Int(0),
        Val::Int(-1),
        Val::Double(i64::MAX as f64),
        Val::Double(9_007_199_254_740_992.0),
        Val::Double(9_007_199_254_740_993.0),
        Val::Double(-0.0),
        Val::Double(0.0),
        Val::Double(0.5),
        Val::Double(f64::INFINITY),
        Val::Double(f64::NAN),
    ];
    for left in &numbers {
        for right in &numbers {
            let ordering = compare_values(left, right).unwrap();
            let equal = Val::values_equal(left, right);
            if let (Val::Double(left), Val::Double(right)) = (left, right)
                && left.is_nan()
                && right.is_nan()
            {
                // A total order must place two NaNs in the same position, and
                // IEEE requires a NaN to differ from itself. The operators
                // read equality from `Val::values_equal`, so `eq` is false
                // here; the ordering position is an implementation detail no
                // operator exposes.
                assert_eq!(ordering, Ordering::Equal, "two NaNs must share a position");
                assert!(!equal, "a NaN is not equal to itself");
                continue;
            }
            assert_eq!(
                ordering == Ordering::Equal,
                equal,
                "ordering and equality disagree for {left:?} vs {right:?}"
            );
        }
    }
}

#[test]
fn large_integers_do_not_collide() {
    // Two distinct integers stay distinct: routing them through f64 made
    // 2^53+1 and 2^53 compare equal.
    let ordering = compare_values(
        &Val::Int(9_007_199_254_740_993),
        &Val::Int(9_007_199_254_740_992),
    )
    .unwrap();
    assert_eq!(ordering, Ordering::Greater);
    assert!(!Val::values_equal(
        &Val::Int(9_007_199_254_740_993),
        &Val::Int(9_007_199_254_740_992)
    ));
}

#[test]
fn i64_max_does_not_match_its_nearest_double() {
    // i64::MAX has no double neighbour: the next double up is 2^63.
    let ordering = compare_values(&Val::Int(i64::MAX), &Val::Double(i64::MAX as f64)).unwrap();
    assert_eq!(ordering, Ordering::Less);
    assert!(!Val::values_equal(
        &Val::Int(i64::MAX),
        &Val::Double(i64::MAX as f64)
    ));
    // And the reverse direction is the mirror image.
    assert_eq!(
        compare_values(&Val::Double(i64::MAX as f64), &Val::Int(i64::MAX)).unwrap(),
        Ordering::Greater
    );
}

#[test]
fn eq_operator_rejects_the_collapsed_pair() {
    // End to end: an `eq` over two literal values must be false when the two
    // values are distinct, and true when they are the same number.
    let evaluate = |left: &str, right: &str| {
        let node = json::parse(&format!(
            r#"{{"op":"eq","args":[{{"lit":{left}}},{{"lit":{right}}}]}}"#
        ))
        .unwrap();
        let values = IndexMap::new();
        let rows = RowSet::empty();
        evaluate_expression(&node, &values, &rows).unwrap()
    };
    assert_eq!(
        evaluate("9007199254740993", "9007199254740992.0"),
        Val::Bool(false)
    );
    assert_eq!(
        evaluate("9007199254740993", "9007199254740993"),
        Val::Bool(true)
    );
    assert_eq!(evaluate("1", "1.0"), Val::Bool(true));
    assert_eq!(evaluate("0", "-0.0"), Val::Bool(true));
}

#[test]
fn unique_items_rejects_signed_zero_pair() {
    // The hash/equality invariant: equal values must hash equally, or the
    // duplicate check never sees them.
    let schema = r#"{"type":"array","uniqueItems":true}"#;
    for instance in [r#"[0,-0.0]"#, r#"[-0.0,0]"#, r#"[0.0,-0]"#] {
        let error = validate_text(schema, instance, "instance")
            .expect_err("{instance} holds one value twice");
        assert!(
            error.message.contains("uniqueItems"),
            "{instance}: expected a uniqueItems error, got {error}"
        );
    }
    // The positive cases must keep passing: distinct values, and a repeated
    // same-signed zero.
    assert!(validate_text(schema, "[0,0.0000001]", "instance").is_ok());
    assert!(validate_text(schema, "[-0.0,-0.0]", "instance").is_err());
}

#[test]
fn unique_items_compares_large_integers_exactly() {
    // Both are integers in JSON, but a double cannot hold 2^53+1: the
    // schema subset used to report them as duplicates.
    assert!(
        validate_text(
            r#"{"uniqueItems":true}"#,
            "[9007199254740993,9007199254740992]",
            "instance"
        )
        .is_ok()
    );
    // Same number written two ways is still a duplicate.
    assert!(validate_text(r#"{"uniqueItems":true}"#, "[1,1.0]", "instance").is_err());
    assert!(
        validate_text(
            r#"{"uniqueItems":true}"#,
            "[9007199254740993,9007199254740993.0]",
            "instance"
        )
        .is_err()
    );
}

#[test]
fn nested_values_use_the_same_numeric_equality() {
    // The rule is recursive: an array of objects holding numbers must not
    // reintroduce the f64 path.
    assert!(
        validate_text(
            r#"{"type":"array","uniqueItems":true}"#,
            r#"[{"x":9007199254740993},{"x":9007199254740992}]"#,
            "instance"
        )
        .is_ok()
    );
    assert!(
        validate_text(
            r#"{"type":"array","uniqueItems":true}"#,
            r#"[{"x":0},{"x":-0.0}]"#,
            "instance"
        )
        .is_err()
    );
}
