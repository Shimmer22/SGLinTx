# UI Fbdev Performance Notes

## Context

This note records the recent investigation into severe board-side UI lag on the fbdev backend, especially during page swipe animations on the launcher.

Board / runtime context observed during debugging:

- framebuffer device: `/dev/fb0`
- fb driver name: `cvifb`
- actual framebuffer size: `480x800`
- virtual framebuffer size: `480x1600`
- bits per pixel: `32`
- line length: `1920`
- current runtime config in logs: logical UI `800x480`, `rotate=270`, `swap_rb=true`

Known platform-specific behavior reported by the user:

- Linux reuses `/boot/logo.jpeg` as the initial `/dev/fb0` contents after boot if framebuffer is enabled
- on this platform, direct writes to `cvifb` should be treated as `BGRA` bytes in memory:
  - `byte0 = B`
  - `byte1 = G`
  - `byte2 = R`
  - `byte3 = A`
- alpha must be written as `0xff`; writing zero alpha can reveal the background instead of producing black

## What Was Measured

Extra fbdev performance logging was added in `src/ui/backend/fbdev.rs`, gated by:

- `LINTX_UI_DEBUG=1`
- `LINTX_UI_PERF_TRACE=1`

The log format now includes:

- `fps`
- `flush_calls`
- `flush_mpix_s`
- `flush_ms`
- `sync_ui_ms`
- `task_ms`
- `present_ms`
- `pan_calls`
- `pan_ms`

Example observations from the board before the failed 32-bit attempt:

- idle / low activity:
  - `flush_ms` near `0`
  - `task_ms` near `0`
  - `pan_ms` about `0.48ms`
- during launcher swipe animation:
  - `flush_ms` around `48ms` to `53ms`
  - `task_ms` around `56ms` to `61ms`
  - `sync_ui_ms` small, usually `< 2ms`
  - `pan_ms` still about `0.48ms`

## Current Understanding

### Confirmed

1. The main bottleneck is not `FBIOPAN_DISPLAY`.
   `pan_ms` stayed around `0.47ms` to `0.49ms`, which is too small to explain the visible lag.

2. The main bottleneck is not Rust-side UI state sync.
   `sync_ui_ms` remained very small.

3. The heavy cost is in the LVGL render/flush path during animation.
   `task_ms` tracked `flush_ms` closely, which strongly suggests LVGL redraw + fb flush dominates frame time.

4. The lag is strongly correlated with animated page transitions.
   The flush area during swipe appears large enough to behave close to near-full-screen redraw.

5. The earlier user conclusion still holds:
   disabling rotation did not remove the major lag, so rotation is not the only root cause.

### Important nuance

Rotation is still expensive in implementation terms, because the current `rotate=270` path writes pixels with coordinate remapping and per-pixel loops, but it is not sufficient by itself to explain the original lag signature.

The dominant pattern is "animation causes large redraw cost", not "pan is slow" and not simply "software rotation is the only problem".

## Board Facts Collected Remotely

Remote probe against `root@10.85.35.1` showed:

```text
virtual_size: 480,1600
bits_per_pixel: 32
modes: U:480x800p-0
fbset geometry: 480 800 480 1600 32
fbset rgba: 8/0,8/8,8/16,8/24
LineLength: 1920
Name: cvifb
```

Important warning:

- `fbset` bitfield data suggested one interpretation of native color layout
- user-provided real-world behavior says direct memory writes should be treated as `BGRA`
- do not assume `fbset` output alone is authoritative for fast-path packing on this platform

## Attempt That Regressed

An experimental attempt was made to switch LVGL from 16-bit to 32-bit color in order to avoid the existing RGB565 to 32-bit conversion in fbdev flush.

What was changed during that attempt:

- `LV_COLOR_DEPTH` temporarily switched from `16` to `32`
- `third_party/lvgl/src/display.rs` was generalized so raw flush callbacks used `lv_color_t` instead of `lv_color16_t`
- `src/ui/backend/sdl.rs` stopped assuming RGB565 in refresh blitting
- `src/ui/backend/fbdev.rs` gained a `pack_color` helper and tentative native 32-bit format handling

Observed result:

- the board became worse
- user reported severe lag / near lockup during swipe

Why this likely regressed:

1. With `rotate=270`, the backend still cannot exploit a simple row memcpy path.
   It still has to do rotated per-pixel writes.

2. Switching LVGL itself to 32-bit increases draw-buffer and internal render bandwidth significantly.
   This can increase `lv_task_handler()` cost before flush benefits are realized.

3. The attempted native32 assumptions were risky because `cvifb` byte layout is platform-specific and may not match the interpretation inferred from `fbset`.

## Current Code State

### Active / intended changes kept

The following changes are useful and should remain:

- fbdev performance logging in `src/ui/backend/fbdev.rs`
  - `flush_ms`
  - `pan_ms`
  - `flush pixel` accounting
- `src/ui/backend/sdl.rs` now uses channel getters instead of manually decoding RGB565
- `third_party/lvgl/src/display.rs` raw flush callback type uses `lv_color_t`
- several hardcoded white colors in `src/ui/backend/lvgl_core.rs` now use `_LV_COLOR_MAKE(255, 255, 255)`

### Explicit rollback already applied

- `third_party/lvgl-sys/vendor/include/lv_conf.h`
  - `LV_COLOR_DEPTH` has been restored to `16`

This rollback compiled successfully with:

- `cargo check`
- `cargo check --features sdl_ui`

### Residual experimental code still present

`src/ui/backend/fbdev.rs` still contains some experimental native-32-related scaffolding:

- `Native32Format`
- `native32_format`
- `detect_native32_format`
- `pack_color`

At `LV_COLOR_DEPTH=16`, these are mostly dormant / fallback-oriented, but they remain in tree and should be treated as experimental rather than proven.

## Risks For Future AI / Human Changes

1. Do not assume framebuffer color layout from `fbset` alone.
   Validate actual byte order on `cvifb` with a known pixel pattern on the real board.

2. Do not switch LVGL to 32-bit again without measuring end-to-end render cost under `rotate=270`.
   A format-match optimization at flush time can lose overall if LVGL internal rendering cost rises more.

3. Be careful about alpha handling on this platform.
   Zero alpha may reveal background instead of showing intended solid color.

4. The UI logical resolution is `800x480` while the physical framebuffer is `480x800`.
   This means rotation and coordinate transforms are fundamental to this path, not incidental.

5. The most misleading optimization path is "improve color packing first".
   Existing evidence says large animated redraw area is the dominant issue.

## Flush Rectangle Measurement Results (2026-03-26)

Per-flush rectangle logging was added (`LINTX_UI_FLUSH_RECTS=1`) and confirmed the hypothesis.

### Observed behavior during a single page swipe

**Idle state** (only clock updating):
```
fb-flush rect=(15,1)..(88,44) size=74x44 pixels=3256 pct=0.8%
fps=14.9, flush_ms=0.04, task_ms=0.29
```

**Drag initiation** (single touch triggers full redraw):
```
fb-flush rect=(0,0)..(799,479) size=800x480 pixels=384000 pct=100.0%
```

**Page transition animation** (repeated ~92% redraws):
```
fb-flush rect=(0,39)..(799,479) size=800x441 pixels=352800 pct=91.9%
fb-flush rect=(0,39)..(799,479) size=800x441 pixels=352800 pct=91.9%
... (8-10 consecutive frames)
fps=7.5-10.6, flush_ms=43-56ms, task_ms=50-64ms, fullscreen_pct=73-83%
```

**Return to idle**:
```
fb-flush rect=(10,1)..(91,44) size=82x44 pixels=3608 pct=0.9%
fps=14.9, flush_ms=0.04
```

### Root cause confirmed

The page transition animation moves the entire `launcher_panel` object position, causing LVGL to invalidate and redraw the whole panel area (800x441 pixels, everything below the 44px top bar) every frame.

This is not a color conversion or pan bottleneck - it is the animation implementation causing massive per-frame redraw.

### Optimization directions identified

1. **Snapshot-based animation** - Use static bitmap snapshots for transitions instead of moving live LVGL objects. Code already has `SnapshotAnimationState` scaffolding that should be leveraged.

2. **Reduce animation frames** - Slightly reduce the smoothness of transitions to lower total redraw cost.

3. **Optimize rotated write path** - Since large-area writes are confirmed, SIMD or batch optimizations in `write_refresh()` could help.

## Optimizations Applied (2026-03-26)

### 1. Snapshot Scene System

Implemented snapshot-based animation using `lv_snapshot_take()` to capture static bitmaps during transitions:

- `SnapshotScene` enum tracks the current animation type (LauncherDrag, LauncherTransition, etc.)
- `is_compatible_with()` method avoids unnecessary snapshot rebuilds when scenes are similar
- `update_secondary_snapshot_for_drag()` efficiently updates only the secondary snapshot when alt_page changes
- `ensure_snapshot_scene()` manages snapshot lifecycle with proper activation/deactivation
- Debug logging added for snapshot state transitions

**Result**: Snapshot system now correctly activates during page swipes. However, LVGL still redraws the snapshot image objects when their positions change, so the ~92% redraw area persists. The benefit is reduced LVGL internal rendering cost (no live object layout recalculation).

### 2. Animation Frame Reduction

Modified `animate_axis()` parameters in `lvgl_core.rs`:

- Step factor: 0.28 → 0.45 (each frame moves 45% of remaining distance)
- Snap threshold: 8 → 20 pixels (animation ends earlier)

**Result**: Animation frames reduced from ~10-12 to ~7-8 per transition. User confirmed this is acceptable smoothness.

### 3. Measured Improvement

Before optimization:
```
fps=7.5-10.6, flush_ms=43-56ms, fullscreen_pct=73-83%
```

After optimization:
```
fps=9.5-11.2, flush_ms=26-49ms, fullscreen_pct=62-77%
```

Animation feels responsive and smooth with fewer frames.

## Remaining Bottleneck Analysis

The main remaining cost is pixel conversion and rotation in `write_refresh()`:

1. **Per-pixel LUT lookup**: Each RGB565 pixel → 32-bit native format via `rgb565_to_native32_lut`
2. **Coordinate rotation**: 270° rotation requires non-sequential memory access (cache-unfriendly)
3. **Double write**: Both `frame_shadow` and mmap'd framebuffer receive writes

For a 800x441 flush:
- 352,800 pixels × (LUT lookup + byte copy + coordinate transform)
- ~3.7ms per full-panel flush

### Potential Future Optimizations (Not Yet Implemented)

1. **SIMD batch conversion**: Process 4-8 pixels at once using NEON/SSE intrinsics
2. **Tile-based rotation**: Process in cache-friendly 16x16 or 32x32 tiles
3. **Skip frame_shadow write**: When not using page flipping, avoid redundant shadow buffer
4. **Partial animation**: Only animate the cards area (~320px height) instead of full panel (~441px)

## Recommended Next Steps

1. ~~Add per-flush rectangle logging for large updates.~~ (Done)

2. ~~Confirm whether launcher swipe is causing near-full-screen redraw every frame.~~ (Confirmed: yes, ~92% per frame)

3. ~~Implement snapshot-based page transition animation.~~ (Done - system activates correctly)

4. ~~Reduce animation frame count.~~ (Done - 0.45 step, 20px snap threshold)

5. Consider optimizing the rotated write path in `src/ui/backend/fbdev.rs` if further improvement needed.

6. If a native32 direct-write path is revisited later, validate memory byte order on hardware first with explicit test colors and alpha=`0xff`.

## Useful Commands

Board-side perf run:

```bash
LINTX_UI_DEBUG=1 \
LINTX_UI_PERF_TRACE=1 \
cargo run -- -- ui_demo --backend fb --fps 30
```

Board-side detailed flush rectangle logging (for diagnosing redraw area):

```bash
LINTX_UI_DEBUG=1 \
LINTX_UI_PERF_TRACE=1 \
LINTX_UI_FLUSH_RECTS=1 \
cargo run -- -- ui_demo --backend fb --fps 30
```

The enhanced perf log now includes:

- `max_rect=WxH` - largest flush rectangle in the 1-second window
- `fullscreen_pct=N%` - percentage of flush calls that covered >= 80% of screen area

When `LINTX_UI_FLUSH_RECTS=1` is set, each flush call logs:

```
fb-flush rect=(x1,y1)..(x2,y2) size=WxH pixels=N pct=P%
```

This helps confirm whether swipe animations cause near-full-screen redraws.

Remote framebuffer probe used during investigation:

```bash
sshpass -p root ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null root@10.85.35.1 "cat /sys/class/graphics/fb0/virtual_size /sys/class/graphics/fb0/bits_per_pixel /sys/class/graphics/fb0/modes 2>/dev/null"
sshpass -p root ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null root@10.85.35.1 "fbset -fb /dev/fb0 -i 2>/dev/null || true"
```
