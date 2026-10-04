# student-planner-leptos: rules for Copilot

## Project
Student daily-task planner (TickTick-style) in Rust. Full-stack Leptos 0.8 + Axum 0.8
(SSR + hydration), built with cargo-leptos 0.3.11, deployed to Render with Docker.
It is a port of an older Spring Boot + vanilla JS app. A read-only copy is in `draft/`
(gitignored). Treat it as the behavior spec. Never edit anything in `draft/`.

## Layout
- `src/lib.rs` module list, `src/main.rs` server entry (ssr only), `src/app.rs` UI
- `src/slices.rs` DONE and tested. Do not change its logic.
- `style/main.scss` styles, `public/` static assets, `end2end/` browser tests

## Two compile targets (most common source of errors)
Everything in `src/` except `main.rs` and `#[cfg(feature = "ssr")]` blocks also compiles
to wasm32. Shared code must not use std::fs, tokio, sqlx, or system clocks.
Put server-only code behind `#[cfg(feature = "ssr")]`.

## Leptos 0.8 API (do not use older APIs)
Use `use leptos::prelude::*;`, `signal()`, `RwSignal`, `Resource::new`, `Action::new`,
`#[component]`, `#[server]`. Do NOT use `create_signal`, `create_resource`,
`create_action`, `use leptos::*;`, or `cx: Scope`. Axum 0.8 path params are `/{id}`.

## Scope rules
- Edit only the files the task names. When you create a module, add its `pub mod x;`
  line to `src/lib.rs` and nothing else there.
- Do not touch Cargo.toml, Cargo.lock, rust-toolchain.toml, main.rs, or slices.rs
  unless the task names them. Do not add dependencies unless the task lists them.
- Never run git commit, push, or branch commands. Never run `cargo leptos watch/build`
  (the developer already has it running).

## Code style
- Times are minutes since midnight as `u32` (08:00 = 480). Overnight values exceed 1440.
- No `unwrap()` or `expect()` outside tests. Doc comment on every `pub` item.
- Each module ends with `#[cfg(test)] mod tests`. Every behavior you port gets a test.

## Verify loop (run after every change, in this order)
Use a separate target dir so you never block the developer's running build:

    export CARGO_TARGET_DIR=/tmp/copilot-target
    cargo fmt --all
    cargo check --features ssr
    cargo check --lib --features hydrate --target wasm32-unknown-unknown
    cargo clippy --features ssr -- -D warnings
    cargo test --lib --features ssr

Ignore this known noise: the "will be rejected by a future version of Rust" warning
about attribute-derive-macro / proc-macro-error2.

## Done means
All five commands exit 0 with no new warnings. If a command fails twice for the same
reason, STOP and report the exact error. Do not widen the change to work around it.
If the draft is ambiguous, port its observable behavior, add a test that pins your
choice, and list the choice in your summary.

## Final report format
Files changed, each command and its result, and any assumptions made.