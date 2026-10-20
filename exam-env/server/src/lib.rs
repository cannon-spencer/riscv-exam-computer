#![recursion_limit = "256"]

pub mod app;
pub mod model;

#[cfg(feature = "ssr")]
pub mod backend;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
