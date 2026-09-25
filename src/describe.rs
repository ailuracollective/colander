//! The described form: the field index of a compiled triple.
//!
//! `colander_compile` canonicalises the three documents and returns them as
//! text. A caller that then wants to know which field is which has to parse that
//! text again and rebuild facts the core already computed. This module reads the
//! index out of the compiled form instead, so the identifier, the pointer and
//! the per-field flags all come from the structures that validated them.
//!
//! The index used is [`index::build_by_id`], which holds every field including a
//! group, in depth-first document order, so the description needs no traversal of
//! its own: the order is the index's, and the parent link is a property of the
//! pointer the index already holds.
//!
//! [`index::build_by_code`] contributes nothing here. It deliberately omits a
//! group, because a group is not an answer key (`documents.md`), and the
//! description reports every field.
//!
//! What the description deliberately omits:
//!
//! - **Presentation.** A field's `title` and `description` are inert by
//!   `documents.md`, so they stay the caller's. A layout node's title *is*
//!   modelled, and travels in the compiled `uiSchemaJson`.
//! - **The evaluated state.** `visibility`, `enabled`, `required` and
//!   `calculatedValues` belong to `colander_evaluate_rules`, so a caller is
//!   never handed two sources for one fact.
//! - **Component provenance.** `compile` does not retain which field came from
//!   which `component-ref`, so nothing here can report it.

use crate::compile::FormCompilationResult;
use crate::error::Result;
use crate::index;
use crate::json::{self, Json, JsonMap};
use crate::keys::schema_json_keys;

/// The pointer prefix every field path starts at, and the one path that is not
/// itself a field.
fn root_path() -> String {
    format!("/{}", schema_json_keys::FIELDS)
}

/// Describes a compiled triple as a flat list of fields in document order.
pub fn describe(compiled: &FormCompilationResult) -> Result<Json> {
    let form_root = json::parse_object(&compiled.form_schema_json, "form schema")?;
    let by_id = index::build_by_id(&form_root)?;

    let entries = by_id
        .values()
        .map(|info| {
            let mut out = JsonMap::new();
            out.insert("id".to_string(), Json::String(info.id.clone()));
            out.insert("code".to_string(), Json::String(info.code.clone()));
            out.insert("path".to_string(), Json::String(info.path.clone()));
            out.insert(
                "parentPath".to_string(),
                match parent_path(&info.path) {
                    Some(path) => Json::String(path.to_string()),
                    None => Json::Null,
                },
            );
            out.insert("type".to_string(), Json::String(info.field_type.clone()));
            out.insert("required".to_string(), Json::Bool(info.required));
            out.insert("readOnly".to_string(), Json::Bool(info.read_only));
            Json::Object(out)
        })
        .collect();

    let mut out = JsonMap::new();
    out.insert("fields".to_string(), Json::Array(entries));
    out.insert(
        "contentHash".to_string(),
        Json::String(compiled.content_hash.clone()),
    );
    Ok(Json::Object(out))
}

/// The pointer of the field's container, or `None` at the top level.
///
/// A nested field's pointer is its container's pointer plus `/items/<n>`, so
/// the parent is the prefix before the **last** `/items/`, not before the last
/// separator. A top-level field's pointer is `/fields/<n>` and carries no
/// `/items/`, so it has no field parent.
fn parent_path(path: &str) -> Option<&str> {
    let cut = path.rfind("/items/")?;
    let parent = &path[..cut];
    if parent == root_path() {
        None
    } else {
        Some(parent)
    }
}

#[cfg(test)]
mod tests {
    use super::parent_path;

    /// The parent is the prefix before the last `/items/`, not before the last
    /// separator. A first draft truncated one segment and reported
    /// `/fields/0/items` as the parent of `/fields/0/items/0`.
    #[test]
    fn parent_path_strips_the_whole_items_segment() {
        assert_eq!(parent_path("/fields/0/items/1"), Some("/fields/0"));
        assert_eq!(
            parent_path("/fields/0/items/1/items/2"),
            Some("/fields/0/items/1")
        );
    }

    /// A top-level field's pointer carries no `/items/`.
    #[test]
    fn parent_path_of_a_top_level_field_is_none() {
        assert_eq!(parent_path("/fields/0"), None);
        assert_eq!(parent_path("/fields/12"), None);
    }

    /// The `fields` array itself is not a field, so nothing nests under it.
    #[test]
    fn parent_path_of_the_fields_array_is_none() {
        assert_eq!(parent_path("/fields/items/0"), None);
    }
}
