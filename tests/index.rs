use colander::index::*;
use colander::json::{self, JsonMap};

fn root(text: &str) -> JsonMap {
    json::parse_object(text, "form schema").unwrap()
}

#[test]
fn indexes_nested_items_by_id() {
    let index = build_by_id(&root(
        r#"{"fields":[{"id":"g","code":"g","type":"group","items":[
             {"id":"c","code":"g.c","type":"text"}]}]}"#,
    ))
    .unwrap();
    assert_eq!(index.len(), 2);
    assert_eq!(index["c"].path, "/fields/0/items/0");
}

#[test]
fn group_children_are_top_level_in_the_answer_index() {
    let index = build_answer_index(&root(
        r#"{"fields":[{"id":"g","code":"g","type":"group","items":[
             {"id":"c","code":"g.c","type":"text"}]}]}"#,
    ))
    .unwrap();
    assert_eq!(index.len(), 1);
    assert_eq!(index["g.c"].path, "/fields/0/items/0");
}

#[test]
fn repeater_children_are_nested() {
    let index = build_answer_index(&root(
        r#"{"fields":[{"id":"r","code":"r","type":"repeater","items":[
             {"id":"c","code":"r.c","type":"text"}]}]}"#,
    ))
    .unwrap();
    assert_eq!(index["r"].children.len(), 1);
    assert_eq!(index["r"].children[0].code, "r.c");
}
