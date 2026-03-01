# Repository Guidelines

## Project Structure & Module Organization
`src/` contains the LinTx application entrypoint and runtime modules (input, mixer, UI, transport). UI code is organized under `src/ui/` (`app`, `backend`, `model`, `catalog`, `input`).  
`rpos/` is a local crate for scheduling, messaging, and client/server primitives used by LinTx.  
`third_party/` vendors patched dependencies (`joydev-sys`, `lvgl-sys`); only modify these when intentionally updating patches.  
`docs/` stores operational notes such as `docs/KNOWN_ISSUES.md`. Root `.toml` and `.sh` files are runtime config and device setup helpers.

## Build, Test, and Development Commands
- `cargo check` : fast validation for the host target.
- `cargo check --features sdl_ui` : validate LVGL/SDL UI path.
- `cargo check --target x86_64-pc-windows-gnu` : verify Windows build.
- `cross build --target riscv64gc-unknown-linux-musl --release` : produce board binary.
- `cargo run -- --server` : start Unix socket server (`./rpsocket`).
- `cargo run -- -- ui_demo --backend sdl --width 800 --height 480 --fps 30` : launch SDL UI client.

## Coding Style & Naming Conventions
Follow Rust 2021 defaults and keep code `rustfmt`-clean (`cargo fmt`). Use 4-space indentation and trailing commas in multiline literals.  
Use `snake_case` for functions/modules/files (`mock_joystick.rs`), `CamelCase` for types (`MixerOutMsg`), and `SCREAMING_SNAKE_CASE` for constants (`CALIBRATE_FILENAME`).  
Prefer small modules with explicit responsibility and register runtime modules through `rpos::module::Module::register`.

## Testing Guidelines
Primary tests are Rust unit tests inside each module (`#[cfg(test)] mod tests`). Run:
- `cargo test` : workspace unit tests.
- `cargo test -p rpos` : focus `rpos` internals.
- `cargo bench -p rpos --bench schedule_bench` : scheduler benchmark (Criterion).
Name tests by behavior (`test_channel_out`, `test_*_parse`, `test_*_flow`) and cover normal path plus boundary values.

## Commit & Pull Request Guidelines
Use concise Conventional Commit-style messages as seen in history: `feat(ui): ...`, `refactor: ...`, `ui: ...`.  
PRs should include:
- What changed and why.
- Target(s) validated (`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-gnu`, `riscv64gc-unknown-linux-musl`).
- Commands run for validation.
- UI evidence (screenshot/log snippet) for `ui_demo` changes.
- Linked issue(s) and known limitations.
