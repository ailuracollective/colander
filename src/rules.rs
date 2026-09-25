//! Rule analysis, expression evaluation and numeric normalization.
//!
//! Evaluation order is part of the contract: calculations run first (in
//! topological dependency order) so predicates and cross-field validations
//! observe calculated values, then per-field predicates, then validations.

mod analyze;
mod evaluate;
mod expression;
mod model;
mod number;
mod refs;
mod rows;
mod shape;
mod value;

pub use analyze::{analyze, validate_dependencies};
pub use evaluate::evaluate;
pub(crate) use evaluate::{evaluate_core, evaluate_values};
pub use expression::compare_values;
pub use expression::evaluate_expression;
pub use model::{FormRuleEvaluationResult, RuleDependencyMetadata, RuleValidationError};
pub use number::normalize_calculated_value;
pub use refs::collect_references;
pub use rows::RowSet;
pub use value::{EPSILON, Val};
