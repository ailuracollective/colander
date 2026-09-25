//! The rule engine's result types.

use indexmap::IndexMap;

use super::value::Val;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleValidationError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuleDependencyMetadata {
    pub calculated_field_ids: Vec<String>,
    pub evaluation_order: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormRuleEvaluationResult {
    pub visibility: IndexMap<String, bool>,
    pub enabled: IndexMap<String, bool>,
    pub required: IndexMap<String, bool>,
    pub calculated_values: IndexMap<String, Val>,
    pub validation_errors: Vec<RuleValidationError>,
}
