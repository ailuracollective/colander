//! Result types of response validation.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormResponseValidationMode {
    Draft,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormResponseFieldError {
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormResponseValidationResult {
    pub normalized_answers_json: String,
    pub errors: Vec<FormResponseFieldError>,
}

impl FormResponseValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}
