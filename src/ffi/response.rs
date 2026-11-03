//! `colander_validate_response`.

use std::ffi::c_char;

use crate::codec::json::JsonCodec;
use crate::error::ColanderError;
use crate::validate;

use super::envelope::{dispatch, optional_string, require_string};

/// `colander_validate_response`.
///
/// # Safety
/// `request` must be a valid NUL-terminated UTF-8 C string (or null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colander_validate_response(request: *const c_char) -> *mut c_char {
    unsafe {
        dispatch(&JsonCodec, request, |request| {
            let form = require_string(request, "formSchemaJson")?;
            let ui = optional_string(request, "uiSchemaJson")?;
            let rules_json = optional_string(request, "rulesSchemaJson")?;
            let answers = require_string(request, "answersJson")?;
            let page_id = optional_string(request, "pageId")?;
            let mode = match optional_string(request, "mode")?
                .as_deref()
                .unwrap_or("Draft")
            {
                "Complete" => validate::FormResponseValidationMode::Complete,
                "Draft" => validate::FormResponseValidationMode::Draft,
                other => {
                    return Err(ColanderError::new(format!(
                        "Unknown validation mode '{other}' (expected 'Draft' or 'Complete')."
                    )));
                }
            };

            let result = match page_id.as_deref() {
                Some(page_id) => {
                    if mode != validate::FormResponseValidationMode::Complete {
                        return Err(ColanderError::new(
                            "UI_PAGE_REQUIRES_COMPLETE: pageId requires mode 'Complete'; a page check is a Complete check.",
                        ));
                    }
                    validate::validate_page(
                        &form,
                        ui.as_deref(),
                        rules_json.as_deref(),
                        &answers,
                        page_id,
                    )?
                }
                None => {
                    validate::validate(&form, ui.as_deref(), rules_json.as_deref(), &answers, mode)?
                }
            };

            Ok(validate::project_result(&result))
        })
    }
}
