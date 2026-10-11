//! Two functions, both JSON in and JSON out. Errors are thrown as strings.
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = requiredMeasurements)]
pub fn required_measurements(problem_json: &str) -> Result<String, String> {
    shinbun_layout::json::required_measurements(problem_json).map_err(|e| e.to_string())
}

/// `options_json` may be an empty string.
#[wasm_bindgen]
pub fn solve(problem_json: &str, table_json: &str, options_json: &str) -> Result<String, String> {
    shinbun_layout::json::solve(problem_json, table_json, options_json).map_err(|e| e.to_string())
}
