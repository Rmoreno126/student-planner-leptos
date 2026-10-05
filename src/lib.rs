#![recursion_limit = "256"]

pub mod api;
pub mod checklist;
#[cfg(feature = "ssr")]
pub mod db;
pub mod editor;
pub mod editor_modal;
pub mod history_view;
pub mod model;
pub mod notes;
pub mod schedule_view;
pub mod slices;
pub mod snippets;
pub mod timeline;

pub mod app;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
