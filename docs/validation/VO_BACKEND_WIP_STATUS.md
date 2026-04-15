# VO Backend WIP Status

## Summary

This branch adds an experimental `vo` UI backend for the SG2002 / CV18xx board path.
It is **not verified as working yet** on the target board.

The current implementation changed the UI display path from the old CPU-side framebuffer
rotation approach to a hardware media pipeline attempt:

- LVGL still renders into an RGB888 staging buffer
- the staging buffer is submitted to `VPSS`
- `VPSS` converts to `NV21`
- `VO` displays the `NV21` frame
- rotation was first attempted on `VO`, then moved to a `VPSS` rotation fallback

## Reference Material Used

The implementation was aligned against these SDK samples and docs:

- `middleware/v2/sample/vo_draw/sample_vo_draw.c`
- `middleware/v2/sample/vdec_bind_vo/sample_vdec_bind_vo.c`
- CV180x/CV181x MPI media processing documentation

Only those references were treated as the stable baseline for the `VO` path.

## What Was Changed

- added a new Rust backend entrypoint: `--backend vo`
- added `src/ui/backend/vo.rs`
- added `src/ui/backend/vo_shim.c`
- updated `build.rs` to compile and link the CVI SDK shim on `riscv64 linux + lvgl_ui`
- updated board scripts to start the UI with the `vo` backend by default
- added debug logs around backend creation, init, present, and shutdown

## Current Runtime Status

This branch still does **not** produce a confirmed working UI on the target board.

Observed board-side behavior during validation:

- the binary builds successfully in the Docker cross toolchain
- the process starts and repeatedly reports successful `vo present`
- kernel logs show `vb_qbuf: VO waitq is full. drop new one.`
- kernel logs later show `cvitask_disp exit`
- with the earlier `VO` rotation path, the board also reported:
  `[cvi-vip][sc] sclr_disp_set_rect: me's pos(0, 0) size(800, 480) out of range(550, 836).`

That `out of range` log is the clearest concrete failure signal found so far.

## Current Hypothesis

The most likely issue is still in the hardware display geometry / rotation contract,
not in LVGL's software flush logic itself.

Based on the current board logs:

- direct `VO` rotation with the current `800x480` UI input is not reliable on this board / SDK
- `VO` queue saturation follows the display path not consuming frames correctly
- moving rotation from `VO` to `VPSS` is a safer next direction, but it has not been confirmed
  as the final fix yet

## Validation Commands Used

Build:

```bash
docker run --rm \
    -v /home/shimmer/LinTx:/home/shimmer/LinTx \
    -w /home/shimmer/LinTx/LinTx_musl \
    -e LINTX_CVI_SDK=/home/shimmer/LinTx/LicheeRV-Nano-Build \
    lintx-riscv64-cross:latest \
    bash -lc 'git config --global --add safe.directory /home/shimmer/LinTx/LinTx_musl && cargo build --target riscv64gc-unknown-linux-musl --release --features lvgl_ui'
```

Board run:

```bash
cd /root/lintx
sh ./scripts/board/test_gui_crsf.sh /dev/ttyS2 115200 stm32 /dev/ttyS0 115200
```

## Next Steps

- re-verify the `VPSS` rotation path on the board with fresh logs
- confirm whether `VO` should consume `panel_size` or rotated `output_size` frames on this SDK
- compare the final frame geometry against the exact `sample_vdec_bind_vo` sendframe behavior
- do not treat this branch as production-ready until the board shows a stable, visible UI
