//! Component resolution state while compiling a triple.

use indexmap::IndexMap;

use crate::error::{ColanderError, Result};
use crate::hash;
use crate::json::{self, Json, JsonMap};
use crate::keys::schema_json_keys;
use crate::rules;
use crate::semver;

use super::ComponentVersionData;
use super::fields::compile_field_array;
use super::schemas::compile_ui_schema;
use super::util::{clone_or_null, present, require_array};

// ---------------------------------------------------------------------------
// Compilation context
// ---------------------------------------------------------------------------

pub(super) struct ResolvedComponentDependency {
    pub(super) code: String,
    pub(super) version: String,
    pub(super) content_hash: String,
    /// The component's source form, dropped after the first expansion:
    /// the compiled fields are memoized, so keeping the source alive for
    /// the whole call only inflates the peak (SPEC P-11).
    pub(super) form_schema_json: Option<String>,
    pub(super) ui_fields: Option<JsonMap>,
    pub(super) layout_children: Option<Vec<Json>>,
}

pub(super) struct CompilationContext<'a> {
    pub(super) components: &'a [ComponentVersionData],
    pub(super) resolution_stack: Vec<String>,
    pub(super) dependencies: IndexMap<String, ResolvedComponentDependency>,
    pub(super) expanded_reference_layouts: IndexMap<String, Json>,
    /// Compiled field array per `(code, version)`, so a component referenced
    /// from N sites expands once and the N-1 remaining sites clone the result
    /// instead of re-resolving and re-verifying (SPEC P-8). Cloning is linear
    /// in the output, which is the irreducible cost of materialising N copies.
    pub(super) compiled_components: IndexMap<String, Vec<Json>>,
    /// Batch indexed by `(code, version)`; two entries with the same key are
    /// rejected when the index is built (SPEC P-10).
    pub(super) component_index: IndexMap<String, usize>,
    pub(super) budget: ExpansionBudget,
    /// Whether resolving a component also verifies its `contentHash` pin
    /// (SPEC P-3). The context that recomputes a component's own triple turns
    /// this off, so the nested resolutions it performs for the bytes do not
    /// recurse back into verification.
    pub(super) verify_hashes: bool,
    /// Digests already computed for a component's own compiled triple, keyed
    /// like `compiled_components`. A pin for that key is verified from this
    /// cache instead of re-expanding the component's sub-tree (SPEC P-9).
    pub(super) verified_pins: IndexMap<String, String>,
}

/// Expansion budget: one shared, monotone count of materialised field nodes
/// and serialized output bytes. Both are checked before allocating, so the
/// bound holds regardless of how deep or wide the reference graph is.
pub(super) struct ExpansionBudget {
    fields: usize,
    bytes: usize,
}

/// Indexes the component batch by `(code, version)`, rejecting a batch that
/// carries the same key twice: which one wins would otherwise depend on the
/// caller's ordering (SPEC P-10).
fn index_components(components: &[ComponentVersionData]) -> Result<IndexMap<String, usize>> {
    let mut index = IndexMap::new();
    for (position, component) in components.iter().enumerate() {
        let key = format!("{}@{}", component.code, component.version);
        if index.insert(key.clone(), position).is_some() {
            return Err(ColanderError::new(format!(
                "COMPONENT_DUPLICATE_VERSION: the components batch lists '{}' twice.",
                component.code
            )));
        }
    }
    Ok(index)
}

impl ExpansionBudget {
    const MAX_FIELDS: usize = 1_000_000;
    const MAX_BYTES: usize = 256 * 1024 * 1024;

    fn new() -> Self {
        Self {
            fields: 0,
            bytes: 0,
        }
    }

    pub(super) fn charge_fields(&mut self, count: usize) -> Result<()> {
        self.fields = self.fields.saturating_add(count);
        if self.fields > Self::MAX_FIELDS {
            return Err(ColanderError::new(format!(
                "COMPONENT_BUDGET_EXCEEDED: expansion materialised more than {} fields.",
                Self::MAX_FIELDS
            )));
        }
        Ok(())
    }

    pub(super) fn charge_bytes(&mut self, count: usize) -> Result<()> {
        self.bytes = self.bytes.saturating_add(count);
        if self.bytes > Self::MAX_BYTES {
            return Err(ColanderError::new(format!(
                "COMPONENT_BUDGET_EXCEEDED: expansion serialised more than {} bytes.",
                Self::MAX_BYTES
            )));
        }
        Ok(())
    }
}

impl<'a> CompilationContext<'a> {
    pub(super) fn new(components: &'a [ComponentVersionData]) -> Result<Self> {
        let component_index = index_components(components)?;
        Ok(Self {
            components,
            resolution_stack: Vec::new(),
            dependencies: IndexMap::new(),
            expanded_reference_layouts: IndexMap::new(),
            compiled_components: IndexMap::new(),
            component_index,
            budget: ExpansionBudget::new(),
            verify_hashes: true,
            verified_pins: IndexMap::new(),
        })
    }

    pub(super) fn resolve(
        &mut self,
        component_code: &str,
        component_version: &str,
        ref_path: &str,
    ) -> Result<&ResolvedComponentDependency> {
        let key = format!("{component_code}@{component_version}");
        if !self.dependencies.contains_key(&key) {
            // Indexed lookup instead of a linear scan per reference: a batch
            // of C components and R references costs O(C + R), not O(C·R)
            // (SPEC P-10).
            let found = self
                .component_index
                .get(&key)
                .map(|&position| &self.components[position])
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
                        "JSON_PARSE_ERROR: Invalid UI schema for component '{component_code}' version '{component_version}'."
                    ))
                })?;
                let ui_root = ui_root.as_object().ok_or_else(|| {
                    ColanderError::new(format!(
                        "JSON_NOT_OBJECT: Invalid UI schema for component '{component_code}' version '{component_version}'."
                    ))
                })?;
                ui_fields = json::get_object(ui_root, schema_json_keys::FIELDS).cloned();
                layout_children = json::get_array(ui_root, schema_json_keys::LAYOUT).cloned();
            }

            if self.verify_hashes {
                let mut verified = std::mem::take(&mut self.verified_pins);
                let result = verify_component_hash(found, self.components, &mut verified);
                self.verified_pins = verified;
                result?;
            }

            self.dependencies.insert(
                key.clone(),
                ResolvedComponentDependency {
                    code: component_code.to_string(),
                    version: component_version.to_string(),
                    content_hash: found.content_hash.clone().unwrap_or_default(),
                    form_schema_json: Some(found.form_schema_json.clone()),
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

// ---------------------------------------------------------------------------
// Component hash pins (SPEC P-3)
// ---------------------------------------------------------------------------

/// Verify a component's `contentHash` against its own compiled triple.
///
/// An absent pin, or an explicit empty string, is not a pin: `docs/entry-points.md`
/// records that a missing `contentHash` becomes the empty string, and an empty
/// string means "unpinned". Only a non-empty pin is recomputed and compared.
///
/// The recomputation is memoized in `verified_pins`: a component resolved from
/// N reference sites is verified once, not N times (SPEC P-9).
fn verify_component_hash(
    component: &ComponentVersionData,
    components: &[ComponentVersionData],
    verified_pins: &mut IndexMap<String, String>,
) -> Result<()> {
    let Some(pin) = component
        .content_hash
        .as_deref()
        .filter(|pin| !pin.is_empty())
    else {
        return Ok(());
    };

    let key = format!("{}@{}", component.code, component.version);
    let recomputed = match verified_pins.get(&key) {
        Some(digest) => digest.clone(),
        None => {
            let digest = compiled_component_hash(component, components)?;
            verified_pins.insert(key, digest.clone());
            digest
        }
    };
    if recomputed != pin {
        return Err(ColanderError::new(format!(
            "COMPONENT_HASH_MISMATCH: component '{}' version '{}' declares contentHash '{pin}' but its compiled triple hashes to '{recomputed}'.",
            component.code, component.version
        )));
    }
    Ok(())
}

/// The SHA-256 of a component compiled on its own.
///
/// The component is treated as the top-level form — its own `component-ref`
/// fields are expanded from the same batch and it carries no rules — so the
/// digest covers the compiled triple, not the source text, and matches what
/// `colander_compile` reports for that component alone.
fn compiled_component_hash(
    component: &ComponentVersionData,
    components: &[ComponentVersionData],
) -> Result<String> {
    let form_root = json::parse_object(
        &component.form_schema_json,
        &format!("component '{}' form schema", component.code),
    )?;
    let ui_root = match &component.ui_schema_json {
        Some(text) => Some(json::parse_object(
            text,
            &format!("component '{}' UI schema", component.code),
        )?),
        None => None,
    };

    let mut context = CompilationContext::new(components)?;
    context.verify_hashes = false;

    let fields_path = format!("/components/{}/fields", component.code);
    let fields = require_array(
        json::get(&form_root, schema_json_keys::FIELDS),
        &fields_path,
    )?;
    let compiled_fields = compile_field_array(fields, &fields_path, &mut context)?;

    let mut compiled_form = JsonMap::new();
    compiled_form.insert(
        schema_json_keys::SCHEMA_VERSION.to_string(),
        clone_or_null(&form_root, schema_json_keys::SCHEMA_VERSION),
    );
    if let Some(schema_uri) = present(&form_root, schema_json_keys::SCHEMA) {
        compiled_form.insert(schema_json_keys::SCHEMA.to_string(), schema_uri.clone());
    }
    compiled_form.insert(
        schema_json_keys::FIELDS.to_string(),
        Json::Array(compiled_fields),
    );
    let compiled_form_json = json::canonical(&Json::Object(compiled_form));

    let compiled_ui_json = match &ui_root {
        Some(root) => Some(json::canonical(&compile_ui_schema(root, &context)?)),
        None => None,
    };

    hash::content_hash(&compiled_form_json, compiled_ui_json.as_deref(), None)
}
