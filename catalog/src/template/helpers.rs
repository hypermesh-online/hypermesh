// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Handlebars helper functions for template rendering.

use handlebars::{Context as HbContext, Handlebars, Helper, HelperResult, Output, RenderContext};

pub fn uuid_helper(
    _: &Helper,
    _: &Handlebars,
    _: &HbContext,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let uuid = uuid::Uuid::new_v4();
    out.write(&uuid.to_string())?;
    Ok(())
}

pub fn date_helper(
    h: &Helper,
    _: &Handlebars,
    _: &HbContext,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let format = h
        .param(0)
        .and_then(|v| v.value().as_str())
        .unwrap_or("%Y-%m-%d %H:%M:%S");
    let now = chrono::Utc::now();
    let formatted = now.format(format).to_string();
    out.write(&formatted)?;
    Ok(())
}

pub fn upper_helper(
    h: &Helper,
    _: &Handlebars,
    _: &HbContext,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    if let Some(param) = h.param(0) {
        if let Some(s) = param.value().as_str() {
            out.write(&s.to_uppercase())?;
        }
    }
    Ok(())
}

pub fn lower_helper(
    h: &Helper,
    _: &Handlebars,
    _: &HbContext,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    if let Some(param) = h.param(0) {
        if let Some(s) = param.value().as_str() {
            out.write(&s.to_lowercase())?;
        }
    }
    Ok(())
}

pub fn replace_helper(
    h: &Helper,
    _: &Handlebars,
    _: &HbContext,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    if let (Some(text), Some(from), Some(to)) = (
        h.param(0).and_then(|v| v.value().as_str()),
        h.param(1).and_then(|v| v.value().as_str()),
        h.param(2).and_then(|v| v.value().as_str()),
    ) {
        let result = text.replace(from, to);
        out.write(&result)?;
    }
    Ok(())
}

pub fn join_helper(
    h: &Helper,
    _: &Handlebars,
    _: &HbContext,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    if let (Some(array), Some(separator)) = (
        h.param(0).and_then(|v| v.value().as_array()),
        h.param(1).and_then(|v| v.value().as_str()),
    ) {
        let strings: Vec<String> = array
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
        let result = strings.join(separator);
        out.write(&result)?;
    }
    Ok(())
}

pub fn default_helper(
    h: &Helper,
    _: &Handlebars,
    ctx: &HbContext,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    if let Some(param) = h.param(0) {
        let var_name = param.value().as_str().unwrap_or("");

        // Try to get the value from context
        if let Some(value) = ctx.data().get(var_name) {
            if !value.is_null() {
                out.write(&value.to_string())?;
                return Ok(());
            }
        }
    }

    // Use default value
    if let Some(default) = h.param(1) {
        out.write(&default.value().to_string())?;
    }

    Ok(())
}

pub fn if_eq_helper(
    h: &Helper,
    _: &Handlebars,
    _ctx: &HbContext,
    _rc: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param1 = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    let param2 = h.param(1).and_then(|v| v.value().as_str()).unwrap_or("");

    if param1 == param2 {
        out.write("true")?;
    } else {
        out.write("false")?;
    }

    Ok(())
}
