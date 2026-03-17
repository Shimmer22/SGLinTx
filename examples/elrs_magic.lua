local dev = ARGS[1] or "/dev/ttyS3"
local baud = tonumber(ARGS[2] or "115200")

log(string.format("open %s @ %d", dev, baud))
local port = uart_open(dev, baud)
port:set_timeout(100)

local packet = crsf.encode(0xEE, 0x2D, bytes_from_hex("EE EA 01 00"))
for _ = 1, 10 do
    port:write(packet)
    sleep_ms(10)
end

log("magic packet sent")
