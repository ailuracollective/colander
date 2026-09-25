//! Types shared by the schema API and the checker.

pub struct PublishedSchemas<'a> {
    pub form_schema: &'a str,
    pub ui_schema: &'a str,
    pub rules_schema: &'a str,
    pub workflow_schema: &'a str,
}

/// One assertion failure: the keyword that failed and a human message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    pub keyword: String,
    pub message: String,
}
