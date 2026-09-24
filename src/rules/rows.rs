//! Row data for repeater-aware rule evaluation (SPEC R-7, R-7a).
//!
//! A repeater's rows never travel inside the flat rule-value map: the map keeps
//! the repeater's count under its code, unchanged, and the rows ride alongside
//! in a `RowSet`. `colander_validate_response` builds it from the submitted
//! answers; `colander_evaluate_rules` builds it from a `List` under a repeater
//! code, which previously had no defined meaning. Both paths produce the same
//! observable R-7 behaviour; how each obtains rows is internal.

use indexmap::IndexMap;

use crate::json::{self, Json, JsonMap};
use crate::keys::{field_type_names, schema_json_keys};

use super::value::Val;

/// A repeater code mapped to its rows; each row maps a child code to its value.
/// Absent means "no row data" (a count was supplied, or there are no rows).
#[derive(Debug, Clone, Default)]
pub struct RowSet {
    rows: IndexMap<String, Vec<IndexMap<String, Val>>>,
}

impl RowSet {
    pub fn empty() -> Self {
        RowSet {
            rows: IndexMap::new(),
        }
    }

    pub fn rows(&self, repeater_code: &str) -> Option<&Vec<IndexMap<String, Val>>> {
        self.rows.get(repeater_code)
    }

    /// The rows of a repeater for writing computed values back into them, so a
    /// later calculation or aggregate observes them. Rows are the primary store
    /// for repeater-child values; the array in the working values is only the
    /// output projection.
    pub fn rows_mut(&mut self, repeater_code: &str) -> Option<&mut Vec<IndexMap<String, Val>>> {
        self.rows.get_mut(repeater_code)
    }

    /// Every repeater code carrying row data, in document order.
    pub fn codes(&self) -> impl Iterator<Item = &str> {
        self.rows.keys().map(String::as_str)
    }

    /// Build from submitted answers: for each repeater, an array answer becomes
    /// rows, one per object element. Non-object elements are skipped; an absent
    /// or non-array answer contributes no rows.
    pub fn from_answers(form_root: &JsonMap, answers: &JsonMap) -> RowSet {
        let mut rows = IndexMap::new();
        for repeater_code in repeater_codes(form_root) {
            let Some(Json::Array(elements)) = answers.get(&repeater_code) else {
                continue;
            };
            let mut row_list = Vec::new();
            for element in elements {
                let Some(object) = element.as_object() else {
                    continue;
                };
                let mut row = IndexMap::new();
                for (child_code, child_value) in object {
                    match Val::from_json_element(child_value) {
                        Ok(converted) => {
                            row.insert(child_code.clone(), converted);
                        }
                        Err(_) => {
                            row.insert(child_code.clone(), Val::Null);
                        }
                    }
                }
                row_list.push(row);
            }
            rows.insert(repeater_code, row_list);
        }
        RowSet { rows }
    }

    /// Build from already-flattened rule values (the FFI path): a `List` under a
    /// repeater code is interpreted as rows. Objects arrive as `Raw` (via
    /// `from_json_node`), so they are parsed back; number spelling is not
    /// preserved, but values are, which is all calculation needs. A `List` of
    /// non-objects, a number, or an absent key contributes no rows.
    pub fn from_values(form_root: &JsonMap, values: &IndexMap<String, Val>) -> RowSet {
        let mut rows = IndexMap::new();
        for repeater_code in repeater_codes(form_root) {
            let Some(Val::List(elements)) = values.get(&repeater_code) else {
                continue;
            };
            let mut row_list = Vec::new();
            for element in elements {
                let Val::Raw(text) = element else {
                    continue;
                };
                let Ok(Json::Object(object)) = json::parse(text) else {
                    continue;
                };
                let mut row = IndexMap::new();
                for (child_code, child_value) in &object {
                    match Val::from_json_element(child_value) {
                        Ok(converted) => {
                            row.insert(child_code.clone(), converted);
                        }
                        Err(_) => {
                            row.insert(child_code.clone(), Val::Null);
                        }
                    }
                }
                row_list.push(row);
            }
            rows.insert(repeater_code, row_list);
        }
        RowSet { rows }
    }

    /// Maps every field id to its innermost enclosing repeater code. Fields with
    /// no repeater ancestor, and repeaters themselves, have no entry.
    pub fn repeater_parents(form_root: &JsonMap) -> IndexMap<String, String> {
        let mut parents = IndexMap::new();
        if let Some(fields) = json::get_array(form_root, schema_json_keys::FIELDS) {
            walk_fields(fields, None, &mut parents);
        }
        parents
    }

    /// Maps every repeater-child field code to its innermost enclosing
    /// repeater code. The analyzer uses it to reject references that would
    /// silently read one arbitrary row (the flattening step keeps only the
    /// last row per child code): a child code may only be referenced from
    /// row scope — a `calculate` of the same repeater's children — or from
    /// an aggregate (`sum`/`count`), never from a predicate, a validation,
    /// or an unrelated calculation.
    pub fn child_repeater_by_code(form_root: &JsonMap) -> IndexMap<String, String> {
        let mut by_code = IndexMap::new();
        if let Some(fields) = json::get_array(form_root, schema_json_keys::FIELDS) {
            walk_child_codes(fields, None, &mut by_code);
        }
        by_code
    }
}

/// Every repeater code in the form, in document order, including nested ones.
fn repeater_codes(form_root: &JsonMap) -> Vec<String> {
    let mut codes = Vec::new();
    if let Some(fields) = json::get_array(form_root, schema_json_keys::FIELDS) {
        collect_repeater_codes(fields, &mut codes);
    }
    codes
}

fn collect_repeater_codes(fields: &[Json], codes: &mut Vec<String>) {
    for field in fields {
        let Some(object) = field.as_object() else {
            continue;
        };
        let is_repeater =
            json::get_str(object, schema_json_keys::TYPE) == Some(field_type_names::REPEATER);
        if is_repeater && let Some(code) = json::get_str(object, schema_json_keys::CODE) {
            codes.push(code.to_string());
        }
        if let Some(items) = json::get_array(object, schema_json_keys::ITEMS) {
            collect_repeater_codes(items, codes);
        }
    }
}

fn walk_fields(fields: &[Json], repeater: Option<&str>, parents: &mut IndexMap<String, String>) {
    for field in fields {
        let Some(object) = field.as_object() else {
            continue;
        };
        let (Some(id), Some(field_type)) = (
            json::get_str(object, schema_json_keys::ID),
            json::get_str(object, schema_json_keys::TYPE),
        ) else {
            continue;
        };
        if field_type == field_type_names::REPEATER {
            if let (Some(code), Some(items)) = (
                json::get_str(object, schema_json_keys::CODE),
                json::get_array(object, schema_json_keys::ITEMS),
            ) {
                walk_fields(items, Some(code), parents);
            }
            continue;
        }
        if let Some(code) = repeater {
            parents.insert(id.to_string(), code.to_string());
        }
        if let Some(items) = json::get_array(object, schema_json_keys::ITEMS) {
            walk_fields(items, repeater, parents);
        }
    }
}

/// Collects `(child code, enclosing repeater code)` pairs; mirrors
/// [`walk_fields`] but keys by answer code instead of field id.
fn walk_child_codes(
    fields: &[Json],
    repeater: Option<&str>,
    by_code: &mut IndexMap<String, String>,
) {
    for field in fields {
        let Some(object) = field.as_object() else {
            continue;
        };
        let (Some(code), Some(field_type)) = (
            json::get_str(object, schema_json_keys::CODE),
            json::get_str(object, schema_json_keys::TYPE),
        ) else {
            continue;
        };
        if field_type == field_type_names::REPEATER {
            if let Some(items) = json::get_array(object, schema_json_keys::ITEMS) {
                walk_child_codes(items, Some(code), by_code);
            }
            continue;
        }
        if let Some(enclosing) = repeater {
            by_code.insert(code.to_string(), enclosing.to_string());
        }
        if let Some(items) = json::get_array(object, schema_json_keys::ITEMS) {
            walk_child_codes(items, repeater, by_code);
        }
    }
}
