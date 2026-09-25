//! The lossless front-end's derived form semantics for rules and response validation.

use indexmap::IndexMap;

use crate::error::Result;
use crate::index::{self, FieldInfo};
use crate::json::{self, Json, JsonMap};
use crate::keys::{field_type_names, schema_json_keys};

#[derive(Debug, Clone)]
pub(crate) struct FormSemantics {
    pub(crate) fields_by_id: IndexMap<String, FieldInfo>,
    pub(crate) fields_by_code: IndexMap<String, FieldInfo>,
    pub(crate) child_repeater_by_code: IndexMap<String, String>,
    pub(crate) repeater_parent_by_id: IndexMap<String, String>,
    pub(crate) repeater_codes: Vec<String>,
}

impl FormSemantics {
    pub(crate) fn from_form(form_root: &JsonMap) -> Result<Self> {
        Self::from_form_with_code_policy(form_root, true)
    }

    pub(crate) fn from_form_unchecked(form_root: &JsonMap) -> Result<Self> {
        Self::from_form_with_code_policy(form_root, false)
    }

    fn from_form_with_code_policy(form_root: &JsonMap, reject_duplicates: bool) -> Result<Self> {
        let fields_by_id = index::build_by_id(form_root)?;
        let fields_by_code = if reject_duplicates {
            index::build_by_code_from_id(&fields_by_id)?
        } else {
            index::build_by_code_from_id_unchecked(&fields_by_id)
        };
        let topology = repeater_topology(form_root);

        Ok(Self {
            fields_by_id,
            fields_by_code,
            child_repeater_by_code: topology.child_repeater_by_code,
            repeater_parent_by_id: topology.repeater_parent_by_id,
            repeater_codes: topology.repeater_codes,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RepeaterTopology {
    pub(crate) child_repeater_by_code: IndexMap<String, String>,
    pub(crate) repeater_parent_by_id: IndexMap<String, String>,
    pub(crate) repeater_codes: Vec<String>,
}

pub(crate) fn repeater_topology(form_root: &JsonMap) -> RepeaterTopology {
    let mut topology = RepeaterTopology::default();
    if let Some(fields) = json::get_array(form_root, schema_json_keys::FIELDS) {
        walk_repeater_topology(fields, None, &mut topology);
    }
    topology
}

fn walk_repeater_topology(
    fields: &[Json],
    repeater: Option<&str>,
    topology: &mut RepeaterTopology,
) {
    for field in fields {
        let Some(object) = field.as_object() else {
            continue;
        };
        let Some(field_type) = json::get_str(object, schema_json_keys::TYPE) else {
            continue;
        };
        let code = json::get_str(object, schema_json_keys::CODE);
        let id = json::get_str(object, schema_json_keys::ID);

        if field_type == field_type_names::REPEATER {
            let Some(code) = code else {
                continue;
            };
            topology.repeater_codes.push(code.to_string());
            if let Some(items) = json::get_array(object, schema_json_keys::ITEMS) {
                walk_repeater_topology(items, Some(code), topology);
            }
            continue;
        }

        if let Some(repeater) = repeater {
            if let Some(id) = id {
                topology
                    .repeater_parent_by_id
                    .insert(id.to_string(), repeater.to_string());
            }
            if let Some(code) = code {
                topology
                    .child_repeater_by_code
                    .insert(code.to_string(), repeater.to_string());
            }
        }
        if let Some(items) = json::get_array(object, schema_json_keys::ITEMS) {
            walk_repeater_topology(items, repeater, topology);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FormSemantics;
    use crate::json::{self, JsonMap};

    fn root(text: &str) -> JsonMap {
        json::parse_object(text, "form schema").unwrap()
    }

    #[test]
    fn indexes_fields_and_repeaters_in_document_order() {
        let semantics = FormSemantics::from_form(&root(
            r#"{"fields":[
                {"id":"before","code":"before","type":"text"},
                {"id":"group","code":"group","type":"group","items":[
                    {"id":"group.child","code":"group.child","type":"text"}]},
                {"id":"lines","code":"lines","type":"repeater","items":[
                    {"id":"qty","code":"qty","type":"integer"},
                    {"id":"price","code":"price","type":"number"},
                    {"id":"total","code":"total","type":"number","readOnly":true}]},
                {"id":"after","code":"after","type":"text"}]}"#,
        ))
        .unwrap();

        assert_eq!(
            semantics
                .fields_by_id
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "before",
                "group",
                "group.child",
                "lines",
                "qty",
                "price",
                "total",
                "after"
            ]
        );
        assert_eq!(
            semantics
                .fields_by_code
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "before",
                "group",
                "group.child",
                "lines",
                "qty",
                "price",
                "total",
                "after"
            ]
        );
        assert_eq!(semantics.repeater_codes, ["lines"]);
        assert_eq!(semantics.child_repeater_by_code["qty"], "lines");
        assert_eq!(semantics.repeater_parent_by_id["total"], "lines");
    }

    #[test]
    fn rejects_duplicate_field_codes() {
        let error = FormSemantics::from_form(&root(
            r#"{"fields":[
                {"id":"a","code":"duplicate","type":"text"},
                {"id":"b","code":"duplicate","type":"number"}]}"#,
        ))
        .expect_err("duplicate codes must be rejected");

        assert!(error.message.starts_with("RULE_DUPLICATE_FIELD_CODE"));
    }
}
