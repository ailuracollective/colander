//! Compiles the form/ui/rules triple, expanding `component-ref` fields.
//!
//! Components are resolved from the batch of [`ComponentVersionData`] the
//! caller passes in. There is no repository, no callback and no I/O in the
//! core; the caller decides what "published" means.

use crate::error::Result;
use crate::hash;
use crate::json::{self, JsonMap};
use crate::semantic::FormSemantics;

mod context;
mod fields;
mod projection;
mod schemas;
mod util;

use context::CompilationContext;
use projection::CompileSemantics;

/// One resolved component version, as handed in by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentVersionData {
    pub code: String,
    pub version: String,
    pub form_schema_json: String,
    pub ui_schema_json: Option<String>,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormCompilationResult {
    pub form_schema_json: String,
    pub ui_schema_json: Option<String>,
    pub rules_schema_json: Option<String>,
    pub dependency_metadata_json: String,
    pub content_hash: String,
}

/// Compile the triple and expand every `component-ref` against `components`.
pub fn compile(
    form_schema_json: &str,
    ui_schema_json: Option<&str>,
    rules_schema_json: Option<&str>,
    components: &[ComponentVersionData],
) -> Result<FormCompilationResult> {
    let form_root = json::parse_object(form_schema_json, "form schema")?;
    let ui_root = match ui_schema_json {
        Some(text) => Some(json::parse_object(text, "UI schema")?),
        None => None,
    };
    let rules_root = match rules_schema_json {
        Some(text) => Some(json::parse_object(text, "rules schema")?),
        None => None,
    };

    compile_from_roots(
        &form_root,
        ui_root.as_ref(),
        rules_root.as_ref(),
        components,
    )
}

fn compile_from_roots(
    form_root: &JsonMap,
    ui_root: Option<&JsonMap>,
    rules_root: Option<&JsonMap>,
    components: &[ComponentVersionData],
) -> Result<FormCompilationResult> {
    let mut context = CompilationContext::new(components)?;
    let inputs = CompileSemantics {
        form_root,
        ui_root,
        rules_root,
    };
    let compiled_form = inputs.project_form(&mut context)?;

    // R-5: a duplicated field code is rejected on the effective (compiled)
    // documents, whether or not a rules document is present. The semantic
    // projection also supplies the field index used by dependency metadata.
    let form_semantics = FormSemantics::from_form(
        compiled_form
            .as_object()
            .expect("compiled form projection is an object"),
    )?;
    let (compiled_ui, compiled_rules) = inputs.project_ui_rules(&mut context)?;
    let documents = projection::CompiledDocuments {
        form: compiled_form,
        ui: compiled_ui,
        rules: compiled_rules,
    };
    let compiled_form_root = documents.form_root();

    let compiled_form_json = json::canonical(&documents.form);
    let compiled_ui_json = documents.ui.as_ref().map(json::canonical);
    let compiled_rules_json = documents.rules.as_ref().map(json::canonical);
    // Charge the final documents too, so the byte budget covers output the
    // field-level charge cannot see (top-level keys, the documents a caller
    // supplied without references).
    context.budget.charge_bytes(
        compiled_form_json.len()
            + compiled_ui_json.as_deref().map(str::len).unwrap_or(0)
            + compiled_rules_json.as_deref().map(str::len).unwrap_or(0),
    )?;

    let dependency_metadata_json = context.build_dependency_metadata_json(
        &form_semantics,
        compiled_form_root,
        documents.rules_root(),
    )?;
    let content_hash = hash::content_hash_from_canonical_documents(
        &compiled_form_json,
        compiled_ui_json.as_deref(),
        compiled_rules_json.as_deref(),
    );

    Ok(FormCompilationResult {
        form_schema_json: compiled_form_json,
        ui_schema_json: compiled_ui_json,
        rules_schema_json: compiled_rules_json,
        dependency_metadata_json,
        content_hash,
    })
}

#[cfg(test)]
mod tests {
    use super::compile_from_roots;
    use crate::hash;
    use crate::json;

    #[test]
    fn prepared_compile_projection_matches_facade_and_hash() {
        let form_text = r#"{"schemaVersion":"1.0.0","fields":[
            {"id":"amount","code":"amount","type":"number","multipleOf":0.10}]}"#;
        let ui_text = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{}}"#;
        let rules_text = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{}}"#;
        let form = json::parse_object(form_text, "form schema").unwrap();
        let ui = json::parse_object(ui_text, "UI schema").unwrap();
        let rules = json::parse_object(rules_text, "rules schema").unwrap();

        let prepared = compile_from_roots(&form, Some(&ui), Some(&rules), &[]).unwrap();
        let facade = super::compile(form_text, Some(ui_text), Some(rules_text), &[]).unwrap();
        assert_eq!(prepared, facade);
        assert!(prepared.form_schema_json.contains(r#""multipleOf":0.10"#));

        assert_eq!(
            prepared.content_hash,
            hash::content_hash_from_canonical_documents(
                &prepared.form_schema_json,
                prepared.ui_schema_json.as_deref(),
                prepared.rules_schema_json.as_deref(),
            )
        );
        assert_eq!(
            prepared.content_hash,
            hash::content_hash(
                &prepared.form_schema_json,
                prepared.ui_schema_json.as_deref(),
                prepared.rules_schema_json.as_deref(),
            )
            .unwrap()
        );
    }

    #[test]
    fn effective_code_dupes_precede_ui_projection_errors() {
        let form = r#"{"schemaVersion":"1.0.0","fields":[
            {"id":"first","code":"duplicate","type":"text"},
            {"id":"second","code":"duplicate","type":"number"}]}"#;
        let ui = r#"{"schemaVersion":"1.0.0","unexpected":true}"#;

        let error = super::compile(form, Some(ui), None, &[]).unwrap_err();
        assert!(
            error.message.starts_with("RULE_DUPLICATE_FIELD_CODE"),
            "{error}"
        );
    }
}
