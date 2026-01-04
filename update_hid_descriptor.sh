#!/bin/sh
# 更新 HID Report Descriptor 以支持 Web Gamepad API

echo "=== Updating HID Report Descriptor for Web Gamepad API ==="
echo ""

G0_DIR="/sys/kernel/config/usb_gadget/g0"
HID_FUNC="$G0_DIR/functions/hid.usb0"

if [ ! -d "$HID_FUNC" ]; then
    echo "Error: HID function not found at $HID_FUNC"
    exit 1
fi

cd "$G0_DIR"

# 1. 解绑 UDC
echo "[1/4] Unbinding UDC..."
UDC_NAME=$(cat UDC)
echo "" > UDC
sleep 1

# 2. 更新 Report Descriptor
# 新的描述符：
# - Usage Page: Generic Desktop (0x01)
# - Usage: Gamepad (0x05) - 这对 Web API 很重要！
# - 8 个按键
# - 4 个轴：X, Y, Z (双向 -127~127), Rz (单向 0~255 用于油门)
echo "[2/4] Updating HID Report Descriptor..."

# 修改 report_length 为 6 字节（增加油门为 0~255）
echo 6 > "$HID_FUNC/report_length"

# 新的 Report Descriptor (hex) - 正确的 6 字节定义
# 格式：
# Byte 0: 8 buttons (bit-packed)
# Byte 1: X axis (-127~127)
# Byte 2: Y axis (-127~127) 
# Byte 3: Z axis (-127~127)
# Byte 4: Rx axis (-127~127) - 备用轴
# Byte 5: Rz axis (0~255) - 油门（单向）
#
# HID Report Descriptor 解析：
# 05 01        Usage Page (Generic Desktop)
# 09 05        Usage (Game Pad)
# A1 01        Collection (Application)
# 05 09          Usage Page (Button)
# 19 01          Usage Minimum (Button 1)
# 29 08          Usage Maximum (Button 8)
# 15 00          Logical Minimum (0)
# 25 01          Logical Maximum (1)
# 75 01          Report Size (1)
# 95 08          Report Count (8)
# 81 02          Input (Data, Variable, Absolute) - 8 buttons
# 05 01          Usage Page (Generic Desktop)
# 09 30          Usage (X)
# 09 31          Usage (Y)
# 09 32          Usage (Z)
# 09 33          Usage (Rx) - 这个之前漏了！
# 15 81          Logical Minimum (-127)
# 25 7F          Logical Maximum (127)
# 75 08          Report Size (8)
# 95 04          Report Count (4) - X, Y, Z, Rx 四个轴
# 81 02          Input (Data, Variable, Absolute)
# 09 35          Usage (Rz) - 油门
# 15 00          Logical Minimum (0)
# 26 FF 00       Logical Maximum (255)
# 75 08          Report Size (8)
# 95 01          Report Count (1)
# 81 02          Input (Data, Variable, Absolute)
# C0           End Collection
echo "05 01 09 05 A1 01 05 09 19 01 29 08 15 00 25 01 75 01 95 08 81 02 05 01 09 30 09 31 09 32 09 33 15 81 25 7F 75 08 95 04 81 02 09 35 15 00 26 FF 00 75 08 95 01 81 02 C0" | xxd -r -ps > "$HID_FUNC/report_desc"

echo "    Report length: $(cat $HID_FUNC/report_length) bytes"
echo "    Report descriptor updated"

# 3. 重新绑定 UDC
echo "[3/4] Rebinding UDC..."
echo "$UDC_NAME" > UDC
sleep 2

# 4. 验证
echo "[4/4] Verifying..."
if [ -c /dev/hidg1 ]; then
    echo "    ✓ /dev/hidg1 exists"
else
    echo "    ✗ /dev/hidg1 not found"
fi

echo ""
echo "=== HID Report Descriptor Updated! ==="
echo ""
echo "Changes:"
echo "  • Report size: 5 → 6 bytes"
echo "  • Added Rx axis (for future use)"
echo "  • Rz axis (throttle): -127~127 → 0~255"
echo ""
echo "Next steps:"
echo "  1. Restart your LinTx application"
echo "  2. The app needs to be updated to send 6-byte reports"
echo ""
echo "Note: You need to update usb_gamepad.rs to match the new format!"
