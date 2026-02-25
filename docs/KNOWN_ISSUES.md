# Known Issues

## 2026-02-25: `ui_demo --backend sdl` client/server startup instability (deferred)

### Symptoms
- Running client command without a ready server can panic with:
  - `src/main.rs:95:51` (`Client::new(...).unwrap()`)
  - error: `ConnectionRefused`.
- In some WSL environments, starting `--server` may immediately exit due to Unix socket permission error.

### Reproduction (observed)
```bash
./target/x86_64-unknown-linux-gnu/debug/LinTx -- ui_demo --backend sdl --width 800 --height 480 --fps 30
```

### Current status
- This startup reliability issue is **not fixed in this commit**.
- Work on this issue is deferred intentionally.

### Notes
- LVGL migration is kept in this branch.
- `lvgl-sys` is vendored and patched locally to avoid a separate crash in debug checks
  (`string_impl.rs::strncmp` null-pointer handling).
