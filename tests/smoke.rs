use colander::json::format_double;

#[test]
fn formats_whole_numbers_without_a_decimal() {
    assert_eq!(format_double(3.0), "3");
}
