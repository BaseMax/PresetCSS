use presetcss_core::{build_css, compile_theme};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn compile(src: &str) -> Result<String, JsValue> {
    let theme = compile_theme(src).map_err(|diags| {
        let msg = diags.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("\n");
        JsValue::from_str(&msg)
    })?;
    Ok(build_css(&theme, None))
}

#[wasm_bindgen]
pub fn validate_preset(src: &str) -> String {
    match compile_theme(src) {
        Ok(_) => String::new(),
        Err(diags) => diags.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("\n"),
    }
}

#[wasm_bindgen]
pub fn default_preset() -> String {
    presetcss_core::DEFAULT_PRESET.to_string()
}
