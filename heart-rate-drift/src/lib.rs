mod heart_rate_drift;

use heart_rate_drift::{combine_hr_with_time, HeartRateDrift};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn calculate_heart_rate_drift(heart_rates: &[i32], times: &[i32]) {
    let drifts = combine_hr_with_time(heart_rates, times);
    log(&format!(
        "Heart rate drift is {:#?}",
        drifts.heart_rate_drift()
    ));
}

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Export a `greet` function from Rust to JavaScript, that alerts a
// hello message.
#[wasm_bindgen]
pub fn greet(name: &str) {
    log(&format!("Hello, {}!", name));
}
