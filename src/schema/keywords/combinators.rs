//! Evaluation of schema combinators.

use crate::json::{self, Json, JsonMap};

use super::super::ErrorSink;
use super::super::check::{EvaluationContext, Stop};
use super::super::model::SchemaError;

impl EvaluationContext<'_> {
    pub(crate) fn check_combinators(
        &mut self,
        schema: &JsonMap,
        instance: &Json,
        errors: &mut ErrorSink,
    ) -> Option<Stop> {
        if let Some(all_of) = json::get_array(schema, "allOf") {
            for sub_schema in all_of {
                if let Some(stop) = self.check_into(sub_schema, instance, errors) {
                    return Some(stop);
                }
            }
        }

        if let Some(any_of) = json::get_array(schema, "anyOf") {
            let mut matched = false;
            for sub_schema in any_of {
                let (branch_errors, stop) = self.evaluate_branch(sub_schema, instance);
                if let Some(stop) = stop {
                    errors.push_stop(stop);
                    return Some(stop);
                }
                if branch_errors.is_empty() {
                    matched = true;
                    break;
                }
            }
            if !matched {
                errors.push(SchemaError {
                    keyword: "anyOf".to_string(),
                    message: "value does not match any of the listed schemas".to_string(),
                });
            }
        }

        if let Some(one_of) = json::get_array(schema, "oneOf") {
            let mut matches = 0;
            for sub_schema in one_of {
                let (branch_errors, stop) = self.evaluate_branch(sub_schema, instance);
                if let Some(stop) = stop {
                    errors.push_stop(stop);
                    return Some(stop);
                }
                if branch_errors.is_empty() {
                    matches += 1;
                }
            }
            if matches != 1 {
                errors.push(SchemaError {
                    keyword: "oneOf".to_string(),
                    message: format!(
                        "value matches {matches} of the listed schemas, expected exactly 1"
                    ),
                });
            }
        }

        if let Some(negated) = json::get(schema, "not") {
            let (negated_errors, stop) = self.evaluate_branch(negated, instance);
            if let Some(stop) = stop {
                errors.push_stop(stop);
                return Some(stop);
            }
            if negated_errors.is_empty() {
                errors.push(SchemaError {
                    keyword: "not".to_string(),
                    message: "value must not match the negated schema".to_string(),
                });
            }
        }

        if let Some(condition) = json::get(schema, "if") {
            let (condition_errors, stop) = self.evaluate_branch(condition, instance);
            if let Some(stop) = stop {
                errors.push_stop(stop);
                return Some(stop);
            }
            if condition_errors.is_empty() {
                if let Some(then_schema) = json::get(schema, "then")
                    && let Some(stop) = self.check_into(then_schema, instance, errors)
                {
                    return Some(stop);
                }
            } else if let Some(else_schema) = json::get(schema, "else")
                && let Some(stop) = self.check_into(else_schema, instance, errors)
            {
                return Some(stop);
            }
        }
        None
    }
}
