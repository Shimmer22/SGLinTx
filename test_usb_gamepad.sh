#!/bin/sh
# LinTx USB Gamepad 测试启动脚本
# 使用 mock_joystick 生成测试数据，通过 mixer 处理后输出到 USB HID 手柄

set -e

echo "=== LinTx USB Gamepad Test ==="

# 1. 配置 USB 复合设备（保留SSH + 添加手柄）
echo "[1/4] Configuring USB Composite Gadget..."
if [ ! -f /dev/hidg0 ]; then
    sh gamepad_composite.sh
    sleep 2
else
    echo "  USB Gadget already configured (/dev/hidg0 exists)"
fi

# 2. 启动 LinTx 服务器
echo "[2/4] Starting LinTx server..."
killall LinTx 2>/dev/null || true
sleep 1
./LinTx --server &
sleep 1

# 3. 启动数据源（mock_joystick）
echo "[3/4] Starting mock joystick (sine wave mode)..."
./LinTx -- mock_joystick --config mock_config.toml &
sleep 1

# 4. 启动 mixer 和 USB 输出
echo "[4/4] Starting mixer and USB gamepad output..."
./LinTx -- mixer &
sleep 1
./LinTx -- usb_gamepad

echo ""
echo "=== All modules started! ==="
echo "Connect this device to your PC via USB."
echo "Your PC should see:"
echo "  - A network interface (for SSH)"
echo "  - A USB gamepad device"
echo ""
echo "Test the gamepad on your PC with tools like:"
echo "  - Linux: jstest /dev/input/js0"
echo "  - Windows: joy.cpl (Game Controllers)"
echo ""
echo "Press Ctrl+C to stop all modules."
