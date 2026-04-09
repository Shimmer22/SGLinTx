# ELRS TX Debug Status

## Current Status

- Current working UART baudrate for the TX module is `115200`.
- `bind` is now confirmed working on the current hardware.
- This checkpoint intentionally keeps only the minimal ELRS `bind` fix.
- Later WiFi exploration was reverted and is not part of this checkpoint.

## Verified Result

The user has verified that `bind` works after the protocol changes in this checkpoint.

Observed device info from logs includes:

- module name `DuplicateTX ESP`
- ELRS version `3.2.1`

This confirms that:

- UART communication with the TX module is working at `115200`
- CRSF parameter traffic is reaching the module
- the corrected ELRS 3.x bind path is accepted by the module

## What Was Wrong

The older implementation assumed a fixed raw bind command frame:

```text
C8 07 32 EE EA 10 01 14 EB
```

That frame is a valid CRSF command frame, but it is not the ELRS 3.x bind path used by the upstream Lua tooling.

For ELRS 3.x, bind is triggered through the parameter protocol:

- discover module parameters
- find the `Bind` command field
- execute it with a `0x2D` parameter command write

## Cross-Checked References

This checkpoint was cross-checked against two references:

- OpenTX Crossfire scripts in `../opentx`
- ExpressLRS `3.2.1` Lua implementation

The important conclusion from those references is:

- ELRS `3.2.1` bind is not a fixed hardcoded raw bind frame
- the TX module uses dynamic parameter fields
- the bind trigger is a command field executed through `0x2D`
- for TX module `0xEE`, the handset/source side should be `0xEF`

## Code Changes Kept In This Checkpoint

Only the minimum bind-related protocol fixes remain.

### 1. Bind now uses the ELRS 3.x parameter command path

Instead of always sending the old raw bind frame, LinTx now:

- looks up the `Bind` command field from discovered parameters
- queues a parameter command frame for that field
- tracks bind progress through the existing bind-active flow

## 2. Extended parameter frame addressing was corrected

For ELRS parameter traffic, the address order was corrected to:

```text
deviceId, handsetId
```

The TX module path now uses:

- `deviceId = 0xEE`
- `handsetId = 0xEF`

This affects:

- `REQUEST_SETTINGS (0x2A)`
- `PARAM_READ (0x2C)`
- `PARAM_WRITE / COMMAND (0x2D)`

## Representative Corrected Frames

Correct parameter read example for TX module field `0x3E`:

```text
C8 06 2C EE EF 3E 00 1A
```

Bind command execution now follows the same ELRS 3.x command path:

```text
0x2D + { 0xEE, 0xEF, field_id, 1 }
```

where `field_id` is the discovered `Bind` command field.

## Files Changed For This Checkpoint

- `src/elrs/mod.rs`
- `docs/elrs_debug_status.md`

No WiFi, UI, logging, or extra parameter-enumeration experiments are kept in this state.

## Validation

Targeted ELRS tests pass:

```text
cargo test elrs:: -- --nocapture
```

At this checkpoint:

- `bind` is confirmed working by on-device testing
- the remaining WiFi investigation is deferred to a later checkpoint
