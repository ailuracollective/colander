//! Schema JSON key names, field type names, and widget names.
//!
//! The exact spelling of every string below is part of compiled output's
//! contract with its consumers: do not rename them.

pub mod schema_json_keys {
    pub const FIELDS: &str = "fields";
    pub const SCHEMA_VERSION: &str = "schemaVersion";
    pub const FORM_SCHEMA_VERSION: &str = "formSchemaVersion";
    pub const SCHEMA: &str = "$schema";
    pub const LAYOUT: &str = "layout";
    pub const VALIDATIONS: &str = "validations";
    pub const FIELD_ID: &str = "fieldId";
    pub const ITEMS: &str = "items";
    pub const CHILDREN: &str = "children";
    pub const LABEL: &str = "label";
    pub const WIDGET: &str = "widget";
    pub const WIDTH: &str = "width";
    pub const ID: &str = "id";
    pub const TYPE: &str = "type";
    pub const CODE: &str = "code";
    pub const OPTIONS: &str = "options";
    pub const CALCULATE: &str = "calculate";
    pub const MESSAGE: &str = "message";
}

pub mod field_type_names {
    pub const TEXT: &str = "text";
    pub const TEXTAREA: &str = "textarea";
    pub const NUMBER: &str = "number";
    pub const INTEGER: &str = "integer";
    pub const BOOLEAN: &str = "boolean";
    pub const DATE: &str = "date";
    pub const DATETIME: &str = "datetime";
    pub const TIME: &str = "time";
    pub const CHOICE: &str = "choice";
    pub const GROUP: &str = "group";
    pub const REPEATER: &str = "repeater";
    pub const COMPONENT_REF: &str = "component-ref";
    pub const FILE: &str = "file";
}

pub mod widget_names {
    pub const TEXT_INPUT: &str = "text-input";
    pub const TEXTAREA: &str = "textarea";
    pub const NUMBER_INPUT: &str = "number-input";
    pub const INTEGER_INPUT: &str = "integer-input";
    pub const TOGGLE: &str = "toggle";
    pub const DATE_PICKER: &str = "date-picker";
    pub const DATETIME_PICKER: &str = "datetime-picker";
    pub const TIME_PICKER: &str = "time-picker";
    pub const SELECT: &str = "select";
    pub const GROUP: &str = "group";
    pub const REPEATER: &str = "repeater";
}
