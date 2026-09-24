//! Component resolution state while compiling a triple.

use indexmap::IndexMap;

use crate::error::{ColanderError, Result};
use crate::json::{self, Json, JsonMap};
use crate::keys::schema_json_keys;
use crate::rules;
use crate::semver;

use super::ComponentVersionData;

// ---------------------------------------------------------------------------
// Compilation context
// ---------------------------------------------------------------------------

pub(super) struct ResolvedComponentDependency {
    pub(super) code: String,
    pub(super) version: String,
    pub(super) content_hash: String,
    pub(super) form_schema_json: String,
    pub(super) ui_fields: Option<JsonMap>,
    pub(super) layout_children: Option<Vec<Json>>,
}

pub(super) struct CompilationContext<'a> {
    pub(super) components: &'a [ComponentVersionData],
    pub(super) resolution_stack: Vec<String>,
    pub(super) dependencies: IndexMap<String, ResolvedComponentDependency>,
    pub(super) expanded_reference_layouts: IndexMap<String, Json>,
}

impl<'a> CompilationContext<'a> {
    pub(super) fn new(components: &'a [ComponentVersionData]) -> Self {
        Self {
            components,
            resolution_stack: Vec::new(),
            dependencies: IndexMap::new(),
            expanded_reference_layouts: IndexMap::new(),
        }
    }

    pub(super) fn resolve(
        &mut self,
        component_code: &str,
        component_version: &str,
        ref_path: &str,
    ) -> Result<&ResolvedComponentDependency> {
        let key = format!("{component_code}@{component_version}");
        if !self.dependencies.contains_key(&key) {
            let found = self
                .components
                .iter()
                .find(|item| item.code == component_code && item.version == component_version)
                .ok_or_else(|| {
                    ColanderError::new(format!(
                        "COMPONENT_VERSION_NOT_FOUND: component '{component_code}' version '{component_version}' referenced at {ref_path} was not found or is not published."
                    ))
                })?;

            let mut ui_fields = None;
            let mut layout_children = None;
            if let Some(ui_json) = &found.ui_schema_json {
                let ui_root = json::parse(ui_json).map_err(|_| {
                    ColanderError::new(format!(
                        "Invalid UI schema for component '{component_code}' version '{component_version}'."
                    ))
                })?;
                let ui_root = ui_root.as_object().ok_or_else(|| {
                    ColanderError::new(format!(
                        "Invalid UI schema for component '{component_code}' version '{component_version}'."
                    ))
                })?;
                ui_fields = json::get_object(ui_root, schema_json_keys::FIELDS).cloned();
                layout_children = json::get_array(ui_root, schema_json_keys::LAYOUT).cloned();
            }

            self.dependencies.insert(
                key.clone(),
                ResolvedComponentDependency {
                    code: component_code.to_string(),
                    version: component_version.to_string(),
                    content_hash: found.content_hash.clone().unwrap_or_default(),
                    form_schema_json: found.form_schema_json.clone(),
                    ui_fields,
                    layout_children,
                },
            );
        }
        Ok(self.dependencies.get(&key).expect("just inserted"))
    }

    pub(super) fn build_dependency_metadata_json(
        &self,
        compiled_form_json: &str,
        compiled_rules_json: Option<&str>,
    ) -> Result<String> {
        let mut ordered: Vec<&ResolvedComponentDependency> = self.dependencies.values().collect();
        ordered.sort_by(|left, right| {
            left.code
                .cmp(&right.code)
                .then_with(|| semver::version_cmp(&left.version, &right.version))
        });

        let components = ordered
            .into_iter()
            .map(|dependency| {
                let mut entry = JsonMap::new();
                entry.insert("code".to_string(), Json::String(dependency.code.clone()));
                entry.insert(
                    "version".to_string(),
                    Json::String(dependency.version.clone()),
                );
                entry.insert(
                    "contentHash".to_string(),
                    Json::String(dependency.content_hash.clone()),
                );
                Json::Object(entry)
            })
            .collect();

        let mut metadata = JsonMap::new();
        metadata.insert("components".to_string(), Json::Array(components));

        if let Some(rules_json) = compiled_rules_json {
            let form_root = json::parse_object(compiled_form_json, "form schema")?;
            let rules_root = json::parse_object(rules_json, "rules schema")?;
            rules::validate_dependencies(&form_root, &rules_root)?;
            let rule_metadata = rules::analyze(&form_root, &rules_root)?;

            let mut rules_entry = JsonMap::new();
            rules_entry.insert(
                "calculatedFieldIds".to_string(),
                Json::Array(
                    rule_metadata
                        .calculated_field_ids
                        .into_iter()
                        .map(Json::String)
                        .collect(),
                ),
            );
            rules_entry.insert(
                "evaluationOrder".to_string(),
                Json::Array(
                    rule_metadata
                        .evaluation_order
                        .into_iter()
                        .map(Json::String)
                        .collect(),
                ),
            );
            metadata.insert("rules".to_string(), Json::Object(rules_entry));
        }

        Ok(json::canonical(&Json::Object(metadata)))
    }
}
