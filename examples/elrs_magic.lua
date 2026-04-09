local dev = ARGS[1] or "/dev/ttyS3"
local baud = tonumber(ARGS[2] or "115200")

-- NOTE: This script sends a raw PARAMETER_WRITE (0x2D) frame to the ELRS TX module.
-- The correct source address for an ELRS Lua client is 0xEF (ELRS_HANDSET_ADDRESS),
-- NOT 0xEA (RADIO_ADDRESS). Using 0xEA is only correct for standard Crossfire/EdgeTX
-- command frames; ELRS 3.x parameter protocol expects 0xEF on the host side.
--
-- IMPORTANT: field_id (the 3rd byte of the payload) is NOT fixed. It is discovered
-- dynamically by enumerating module parameters via 0x2C (PARAM_READ). You cannot
-- hardcode field_id=01 and expect it to be the WiFi field on all modules/firmware
-- versions. Use the LinTx WiFi toggle (which discovers the field at runtime) instead
-- of this script for production use. This script is only for low-level debugging.
--
-- Frame format: C8 06 2D EE EF <field_id> <value> <crc>
--   EE = MODULE_ADDRESS  (destination: TX module)
--   EF = ELRS_HANDSET_ADDRESS  (source: ELRS Lua client)
--   value = 0x01 (COMMAND step 1 = CLICK / execute)

log(string.format("open %s @ %d", dev, baud))
local port = uart_open(dev, baud)
port:set_timeout(100)

-- Replace <field_id> byte (0x01 below) with the actual discovered WiFi command field id.
-- Example: if "Enable WiFi" was found at field id 0x0C during parameter enumeration,
-- change the third byte from 0x01 to 0x0C.
local packet = crsf.encode(0xEE, 0x2D, bytes_from_hex("EE EF 01 01"))
for _ = 1, 10 do
    port:write(packet)
    sleep_ms(10)
end

log("magic packet sent")
