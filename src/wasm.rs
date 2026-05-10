use crate::parse;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = parsePiyolog)]
pub fn parse_piyolog_wasm(input: &str) -> std::result::Result<JsValue, JsValue> {
    let parsed = parse(input).map_err(|error| JsValue::from_str(&error.to_string()))?;
    serde_wasm_bindgen::to_value(&parsed).map_err(|error| JsValue::from_str(&error.to_string()))
}

#[wasm_bindgen(js_name = parsePiyologJson)]
pub fn parse_piyolog_json_wasm(input: &str) -> std::result::Result<String, JsValue> {
    let parsed = parse(input).map_err(|error| JsValue::from_str(&error.to_string()))?;
    serde_json::to_string(&parsed).map_err(|error| JsValue::from_str(&error.to_string()))
}
