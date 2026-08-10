//! Browser boundary for the GloriChess Rust runtime.
#![forbid(unsafe_code)]

use wasm_bindgen::prelude::*;

/// Confirms that the WASM package is loaded. Gameplay bindings are added in a later phase.
#[wasm_bindgen]
pub fn runtime_name() -> &'static str {
    "glorichess"
}
