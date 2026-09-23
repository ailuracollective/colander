use colander::json::format_double;

#[test]
fn crate_links() {
    assert_eq!(format_double(3.0), "3");
}
