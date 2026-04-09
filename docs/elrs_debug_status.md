# ELRS TX Debug Status

## Current Status

- `RF output` logic is working.
- `bind` is still not confirmed working.
- `module wifi` is still not working.
- Current working UART baudrate for the TX module is `115200`.
- `420000` did not show usable responses in current tests.
- After the latest scheduler changes, `DEVICE_INFO` now appears quickly and reliably after `RF output` is enabled.

## Confirmed Good

- RF defaults to `OFF`.
- RF UART opens only after enabling `RF output`.
- Disabling `RF output` closes UART and stops RF traffic.
- Startup `magicnum` packet has been removed.
- `DEVICE_INFO` from TX module is reachable at `115200`.
- Bind frame format has been corrected to match EdgeTX style.
- `ModelID` is now sent immediately after RF enable and is no longer fixed to `0`.
- RC channel traffic is now rate-limited and no longer floods the UART on every mixer update.

## Confirmed From Logs

At `115200`, the TX module responds with `DEVICE_INFO`, for example:

- `DuplicateTX ESP`
- version `3.2.1`

This means:

- UART wiring is basically correct.
- Baudrate `115200` is correct for the current setup.
- CRSF communication is partially working.

## Current Known Problems

### 1. Bind still does not complete

Current bind frame being sent:

```text
C8 07 32 EE EA 10 01 14 EB
```

This frame was previously wrong and has already been corrected.

Remaining issue:

- The frame is sent, but receiver binding still does not complete.
- Current implementation sends a single bind frame and now keeps a short post-bind quiet window.
- It is still unclear whether this module needs:
  - a longer bind hold behavior,
  - an even quieter post-bind window,
  - or some additional prerequisite state.
- Current test evidence suggests the TX module may still effectively be operating in phrase-based behavior, because:
  - a RX configured with the same phrase can connect,
  - a RX with no phrase, left in legacy bind mode, still does not bind,
  - and LinTx still cannot write the module's actual bind phrase because parameter enumeration is not working.

### 2. WiFi parameter is unavailable

Current UI symptom:

- `Module WiFi` always stays `OFF`
- Enter / left / right reports `WiFi command unavailable`

Direct protocol reason:

- We can receive `0x29 DEVICE_INFO`
- But we are not receiving any `0x2B PARAM ENTRY`

That means:

- Module discovery works
- Parameter tree enumeration does not work yet
- Therefore labels like `Enable WiFi`, `Enter WiFi`, `Disable WiFi`, `Exit WiFi` are never discovered

## Work Already Done

### RF / UI behavior

- Added `rf_output_enabled` to config
- Default RF state changed to `OFF`
- RF UART is now gated by the ELRS app switch
- UI shows RF state / link state / feedback

### Removed startup side effects

- Removed old startup `magicnum` logic
- Prevented immediate RF serial activity before the user enables RF output

### Bind work

- Corrected bind frame to:

```text
C8 07 32 EE EA 10 01 14 EB
```

- Stopped normal RC channel frame transmission while bind is active
- Added bind feedback state based on telemetry link observation
- Changed bind behavior from multi-burst to single-shot to align more closely with EdgeTX behavior
- Extended bind-active handling to cover a short settle window so RC traffic stays suppressed immediately after bind
- Fixed logging so only the actual bind command frame is logged as `sending bind frame`

### ELRS parameter discovery work

- Added `PING_DEVICES (0x28)`
- Added `DEVICE_INFO (0x29)` parsing
- Added `PARAM ENTRY (0x2B)` parsing
- Added `PARAM READ (0x2C)` sending
- Added `PARAM WRITE (0x2D)` sending
- Added `COMMAND STATUS (0x2E)` parsing
- Added logging for outgoing and incoming ELRS parameter-related frames
- Reduced repetitive log noise from periodic `PING (0x28)` and `PARAM_READ (0x2C)` traffic so useful events are easier to inspect

### EdgeTX alignment work

- Added `ModelID` command before ping/param discovery
- Added `REQUEST_SETTINGS (0x2A)` before parameter field reads
- Changed `ModelID` from a fixed `0` to a stable model-derived value
- Reworked RC transmission toward a periodic scheduler instead of flushing every queued mixer update
- Adjusted protocol initialization order toward:

```text
ModelID -> Ping -> RequestSettings -> FieldRead
```

### Current attempt path

This checkpoint focused on one path only:

- make UART traffic less noisy and more EdgeTX-like
- ensure protocol frames are not starved by RC channel writes
- reduce misleading logs so the real protocol sequence is visible

Concretely this attempt did:

- rate-limit RC channel output to a fixed minimum interval
- keep only the latest mixer state instead of flushing the full backlog
- send protocol frames before RC frames
- keep bind quiet time active after the bind frame is sent

This path solved:

- `DEVICE_INFO` can now be observed almost immediately after enabling RF
- repeated fast bind clicking is no longer required to provoke module responses
- logs are now much easier to read

This path did not solve:

- legacy bind to a no-phrase RX still does not complete
- parameter enumeration still does not return any `0x2B PARAM ENTRY`
- module phrase state still cannot be inspected or changed from LinTx

## Important Observations

### Observation A: `DEVICE_INFO` works, parameter tree does not

Representative log pattern:

```text
rf_link_service serial open ok on /dev/ttyS2 @ 115200 baud
rf_link_service sending ELRS model id on /dev/ttyS2: [C8, 08, 32, EE, EA, 10, 05, 61, 42, 59]
rf_link_service sending ELRS request settings on /dev/ttyS2: [C8, 04, 2A, EA, EE, 96]
received ELRS device info: [EA, 1E, 29, ...]
```

But no:

```text
received ELRS param entry: ...
```

Interpretation:

- The main unresolved issue for WiFi is parameter tree bring-up.
- The scheduler improvements were enough to expose module responses reliably, so the remaining blocker is less likely to be raw UART starvation and more likely to be request format / protocol state.

### Observation B: bind is sent, but immediately followed by other traffic

Representative log pattern:

```text
sending bind frame 1/1 ...
...
received ELRS device info: [EA, 1E, 29, ...]
```

Interpretation:

- This is no longer dominated by RC flood behavior.
- The remaining bind failure now looks more like module-side state mismatch than pure scheduler starvation.
- One strong hypothesis is: the TX module still has an active bind phrase, while the test RX is intentionally left in no-phrase legacy bind mode.

### Observation C: bind phrase edits are still local only

Representative UI/log pattern:

```text
Bind phrase saved locally; Bind phrase parameter unavailable
```

Interpretation:

- LinTx local config can store a phrase string.
- The actual TX module phrase is still not writable because no parameter entries are discovered.
- Therefore current tests that depend on changing or clearing TX phrase from LinTx are not yet valid.

## Probable Remaining Causes

### For WiFi

Most likely causes:

- `0x2A REQUEST_SETTINGS` payload is still not exactly what the module expects
- `0x2C FIELD READ` timing/order is still not what the module expects
- parameter enumeration may require additional handshake or chunk behavior not yet implemented

### For bind

Most likely causes:

- module-side stored bind phrase is still active, so legacy bind against a no-phrase RX is not expected to work
- single-shot bind may still be insufficient for this specific module/firmware combination
- bind may require a different state transition than the current `ModelID -> bind -> quiet -> discovery` sequence

## Current Code State

Main touched files:

- `src/config/mod.rs`
- `src/elrs/mod.rs`
- `src/elrs_tx.rs`
- `src/messages.rs`
- `src/ui/apps/control.rs`
- `src/ui/apps/scripts.rs`
- `src/ui/backend/lvgl_core.rs`

Latest commit for this checkpoint:

- pending new checkpoint commit
- this document now describes the post-scheduler-tuning state, not the earlier `28a68c0` checkpoint

## Recommended Next Steps

Choose one path at a time.

### Path 1: Finish WiFi / parameter tree

Focus only on:

- exact `0x2A REQUEST_SETTINGS` format
- exact `0x2C` read sequence
- whether chunked `0x2B` handling is missing behavior
- compare with additional references such as OpenTX / ELRS Lua behavior, not only EdgeTX `crossfire.cpp`

Success condition:

- log starts showing `received ELRS param entry`

### Path 2: Finish bind behavior

Focus only on:

- validate whether the TX module is still in phrase mode
- test bind only after TX phrase is explicitly cleared outside LinTx
- if phrase is truly cleared, then continue testing bind timing / repetition strategy

Success condition:

- RX successfully binds while already in bind mode with a known no-phrase TX state

## Short Summary For Future Conversation

Use this summary:

- TX module works at `115200`
- `DEVICE_INFO` works
- scheduler tuning made `DEVICE_INFO` appear reliably right after RF enable
- RF on/off logic is correct
- bind frame format is corrected
- bind logging was fixed; ping/read frames are no longer mislabeled as bind
- bind still does not complete
- current evidence suggests TX phrase state may still be the real blocker for legacy bind tests
- WiFi still fails because parameter entries (`0x2B`) never come back
- current likely issue is CRSF parameter-tree handshake/timing plus possible module phrase-state mismatch, not basic UART
