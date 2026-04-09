# ELRS TX Debug Status

## Current Status

- Current working UART baudrate for the TX module is `115200`.
- ELRS `bind` is confirmed working on current hardware.
- ELRS `WiFi enable` path is now root-caused and fixed in protocol/runtime logic.
- Remaining behavior is consistent with ELRS 3.x design:
  - entering WiFi is a parameter command
  - exiting WiFi is not a CRSF command path (module typically needs restart/reconnect path)

---

## Verified Environment and Evidence

From runtime logs:

- Device info frame received:
  - module name: `DuplicateTX ESP`
  - ELRS signature: `ELRS`
  - version bytes: `3.2.1`
  - **param_count = `0x13` (19)**

Representative device info frame:

```text
[EA, 1E, 29, EA, EE, 44, 75, 70, 6C, 65, 54, 58, 20, 45, 53, 50, 00, 45, 4C, 52, 53, 00, 00, 00, 00, 00, 03, 02, 01, 13, 00, D8]
```

Decoded key points:

- name = `DupleTX ESP` (module string in frame)
- serial marker = `ELRS`
- version = `3.2.1`
- **field count = 19**

This confirms UART + CRSF parameter transport is functional and module parameter protocol is active.

---

## What Was Wrong (WiFi Path)

### 1) WiFi “loading forever” symptom

Observed behavior:

- UI command:
  - `Activate -> WiFi params loading, please wait`
- Stays in this state too long / repeatedly without actually entering WiFi.

### 2) Log evidence that exposed the root cause

During settings enumeration:

```text
param entry field=0x0F chunks_rem=1 kind=0x0D label="Enable W" [parsed]
```

And similarly many entries were chunked (`chunks_rem > 0`), including WiFi-related fields.

Key evidence:

- `field=0x0F kind=0x0D` is a **COMMAND** field and is clearly the WiFi command (`Enable WiFi`) but label appears truncated (`Enable W`) in first chunk.
- `field=0x11 kind=0x0D label="Bind"` appears complete and works immediately, matching successful bind path.
- WiFi failed while bind succeeded under same transport, pointing to label/parse matching issue rather than UART issue.

---

## Root Causes (Final)

### Root Cause A — device param scan limit wrong

- Implementation previously used fixed max (`64`) for parameter reads.
- Actual module reported `param_count=19`.
- Over-scanning created unnecessary read traffic and delayed usable state.

### Root Cause B — chunked parameter labels were not handled robustly for WiFi discovery

- ELRS parameter entries can arrive in chunks (8-byte payload chunks).
- WiFi command label may appear as truncated first-chunk text (`"Enable W"`), with remaining text in next chunk.
- Exact-string WiFi lookup (`"Enable WiFi"`, `"Enter WiFi"`, `"WiFi Update"`) could miss truncated labels.

### Root Cause C — fallback matching too strict

- Fallback matching based only on `contains("wifi")` fails on truncated first chunk (`"Enable W"` does not contain `"wifi"`).

---

## Fixes Applied

## 1. Use actual `param_count` from device info

`parse_device_info` now extracts field count from frame (`name_end + 12`) and stores it in `DeviceInfo.field_count`.

Effect:

- Enumeration stops at module-reported count (`19` for this module), not fixed `64`.
- Faster and less noisy parameter discovery.

## 2. Parameter scan upper bound now dynamic

`poll_outgoing_frame` uses:

- `scan_limit = device_info.field_count` (fallback to default max only if unknown).

Effect:

- predictable scan completion
- shorter delay before commands become actionable

## 3. COMMAND entries tolerate truncated first chunk labels

For `PARAM_SETTINGS_ENTRY (0x2B)` parsing:

- non-COMMAND types still require complete chunk semantics
- COMMAND type now allows first chunk parsing even when label terminator is missing in current frame, capturing truncated label safely

Effect:

- COMMAND fields are registered earlier (including WiFi command candidates)
- avoids dropping actionable command entries due to chunk boundary

## 4. WiFi command lookup now has robust fallback

After exact label matching fails, fallback now supports:

- case-insensitive `contains("wifi")`
- **prefix-based matching against known full labels**, so truncated labels like `"Enable W"` can map to `"Enable WiFi"`

Effect:

- WiFi command field (`0x0F`) is discoverable on chunked-label modules

## 5. Runtime state improvements already aligned

- WiFi enable sets local state only when command is actually queued.
- WiFi mode active path suppresses RC frame streaming to avoid unnecessary serial traffic while module is in WiFi behavior.
- RF disable / module reconnect paths clear stale local WiFi state.

---

## Why Bind Worked While WiFi Failed

Bind field log:

```text
field=0x11 kind=0x0D label="Bind"
```

- label is short and complete in one chunk
- exact match succeeds
- command frame sent:

```text
[C8, 06, 2D, EE, EF, 11, 01, A5]
```

WiFi field log:

```text
field=0x0F kind=0x0D label="Enable W"
```

- label truncated in first chunk
- exact match failed before fix
- fallback also failed before prefix support
- therefore UI stayed in “params loading”/unavailable state

---

## Representative Correct Frames

Extended parameter addressing for ELRS TX module remains:

- destination/device: `0xEE`
- handset/source: `0xEF`

Examples:

- Read:
  - `C8 06 2C EE EF <field_id> <chunk> <crc>`
- Command/Write:
  - `C8 06 2D EE EF <field_id> 01 <crc>`

Bind command verified with:

```text
[C8, 06, 2D, EE, EF, 11, 01, A5]
```

---

## Files Changed (This Debug Round)

- `src/elrs/mod.rs`
- `src/elrs_tx.rs`
- `examples/elrs_magic.lua`
- `docs/elrs_debug_status.md`

---

## Validation

Targeted tests:

```text
cargo test elrs:: -- --nocapture
```

Result after fixes:

- ELRS test set passes
- bind flow still good
- WiFi command discovery path no longer blocked by chunked-label edge case
- logs now provide clearer `field/chunk/kind/label` visibility for future debugging

---

## Final Conclusion

WiFi “卡住” was not caused by mixer RC output conflict as primary root cause.  
Primary issue was **parameter discovery robustness** under chunked ELRS 0x2B entries, combined with strict label matching and incorrect scan limit assumptions.

After applying:

- real `param_count` scan limit
- COMMAND chunk-tolerant parsing
- WiFi prefix fallback matching

the WiFi command path is structurally aligned with ELRS 3.2.1 behavior and no longer blocked by the previously observed “Enable W” truncated label case.