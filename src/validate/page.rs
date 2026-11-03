//! Page membership for page-scoped response validation.
//!
//! A page is presentation: a top-level layout node of `type:"page"` whose
//! `children` name the fields it renders. Validation reads that tree only
//! when the caller supplies a `pageId`; the form schema remains the authority
//! on what a field is.

use std::collections::HashSet;

use crate::error::{ColanderError, Result};
use crate::json::{self, Json};
use crate::keys::schema_json_keys;

use super::ellipsize;

/// Field ids rendered by the top-level page whose `id` is `page_id`.
///
/// Only `children` are walked: a repeater's rows are validated through the
/// repeater itself, and pages do not nest.
pub(super) fn page_field_ids(
    ui_schema_json: Option<&str>,
    page_id: &str,
) -> Result<HashSet<String>> {
    let ui_root = ui_schema_json
        .map(|text| json::parse_object(text, "UI schema"))
        .transpose()?;
    let page = ui_root
        .as_ref()
        .and_then(|root| json::get_array(root, schema_json_keys::LAYOUT))
        .and_then(|layout| layout.iter().find(|node| is_page(node, page_id)))
        .and_then(Json::as_object);

    let Some(page) = page else {
        return Err(ColanderError::new(format!(
            "UI_UNKNOWN_PAGE: no page with id '{}' exists in the UI layout.",
            ellipsize(page_id)
        )));
    };

    let mut ids = HashSet::new();
    if let Some(children) = json::get_array(page, schema_json_keys::CHILDREN) {
        collect_field_ids(children, &mut ids);
    }
    Ok(ids)
}

fn is_page(node: &Json, page_id: &str) -> bool {
    node.as_object().is_some_and(|object| {
        json::get_str(object, schema_json_keys::TYPE) == Some("page")
            && json::get_str(object, schema_json_keys::ID) == Some(page_id)
    })
}

fn collect_field_ids(nodes: &[Json], ids: &mut HashSet<String>) {
    // Iterative: a hostile layout must not be able to exhaust the stack.
    let mut stack: Vec<&Json> = nodes.iter().collect();
    while let Some(node) = stack.pop() {
        let Some(object) = node.as_object() else {
            continue;
        };
        if let Some(field_id) = json::get_str(object, schema_json_keys::FIELD_ID) {
            ids.insert(field_id.to_string());
        }
        if let Some(children) = json::get_array(object, schema_json_keys::CHILDREN) {
            stack.extend(children);
        }
    }
}
