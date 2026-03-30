# Repository Guidelines

## Project Structure & Module Organization
`src/` contains the main `LinTx` binary and feature modules such as `elrs_agent`, `mixer`, `adc`, and `ui/` for the LVGL/SDL frontends. `rpos/` is the local runtime/messaging crate used by the app. Board bring-up and deploy helpers live in `scripts/board/`. Reference docs and performance notes are under `docs/`. Vendored dependencies are kept in `third_party/`; treat those as upstream code unless a task explicitly requires patching them.

## Build, Test, and Development Commands
Use Cargo for host builds and `cross` for the SG2002 target.

- `cargo check` validates the default Linux host build.
- `cargo check --features sdl_ui` verifies the desktop UI path.
- `cargo check --features lua` or `--features joydev_input` checks optional modules.
- `cargo test` runs the unit tests in `src/` and `rpos/`.
- `cross build --target riscv64gc-unknown-linux-musl --release --features lvgl_ui` builds the board GUI binary using [`Cross.toml`](/home/shimmer/LinTx/LinTx_musl/Cross.toml).
- `sh scripts/board/test_gui_mock.sh` starts the board-side mock GUI flow after deployment.

## Coding Style & Naming Conventions
This repository is Rust 2021. Follow standard `rustfmt` formatting with 4-space indentation and keep modules/files in `snake_case` (for example `system_state_mock.rs`). Types and traits use `CamelCase`; functions, variables, and feature flags use `snake_case`. Prefer small focused modules and keep board-specific behavior behind feature flags or scripts rather than scattered conditionals.

## Testing Guidelines
Unit tests are colocated with implementation using `#[cfg(test)] mod tests`. Current coverage is concentrated in config, calibration, mixer, ELRS parsing, and `rpos`; add tests next to the code you change. Name tests by behavior, such as `test_radio_and_model_config_roundtrip_toml`. For scheduling work in `rpos`, benchmarks live in [`rpos/benches/`](/home/shimmer/LinTx/LinTx_musl/rpos/benches).

## Commit & Pull Request Guidelines
Recent history follows Conventional Commit style: `feat(ui): ...`, `refactor(ui): ...`, `perf(ui): ...`, `docs: ...`. Keep the scope narrow and descriptive. PRs should explain the user-visible or hardware-visible impact, list the commands run (`cargo test`, feature checks, board scripts), and include screenshots or logs for UI, framebuffer, or board-flow changes.

## Configuration & Environment Tips
Repository configs such as `radio.toml`, `joystick.toml`, and `mock_config.toml` are part of normal development. Avoid committing machine-local secrets or ad hoc device paths. When changing board flows, document required env vars like `LINTX_FB_ROTATE` and `LINTX_FB_SWAP_RB` in the PR.
