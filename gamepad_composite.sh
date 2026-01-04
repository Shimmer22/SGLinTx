#!/bin/sh
# USB Composite Gadget for SG2002: RNDIS Network + HID Gamepad
# 保持SSH连接的同时，提供USB手柄功能

set -e

echo "=== USB Composite Gadget Setup for SG2002 ==="

# 1. 创建并挂载 configfs
CONFIGFS_DIR="/sys/kernel/config"
if [ ! -d "$CONFIGFS_DIR" ]; then
    echo "Error: configfs not available in kernel"
    exit 1
fi

# 检查 configfs 是否已挂载
if ! mountpoint -q "$CONFIGFS_DIR" 2>/dev/null; then
    echo "[1/8] Mounting configfs..."
    mount -t configfs none "$CONFIGFS_DIR" || {
        echo "Error: Failed to mount configfs"
        exit 1
    }
else
    echo "[1/8] configfs already mounted"
fi

# 2. 进入 USB gadget 目录
GADGET_DIR="$CONFIGFS_DIR/usb_gadget"
if [ ! -d "$GADGET_DIR" ]; then
    echo "Error: USB Gadget not supported in this kernel"
    exit 1
fi

cd "$GADGET_DIR"
echo "[2/8] Entered USB gadget directory"

# 3. 清理旧配置
GADGET_NAME="LinTxComposite"
if [ -d "$GADGET_NAME" ]; then
    echo "[3/8] Cleaning up old configuration..."
    cd "$GADGET_NAME"
    
    # 禁用 gadget
    echo "" > UDC 2>/dev/null || true
    
    # 移除函数链接
    rm -f configs/c.1/rndis.usb0 2>/dev/null || true
    rm -f configs/c.1/hid.usb0 2>/dev/null || true
    
    # 移除配置
    rmdir configs/c.1/strings/0x409 2>/dev/null || true
    rmdir configs/c.1 2>/dev/null || true
    
    # 移除函数
    rmdir functions/rndis.usb0 2>/dev/null || true
    rmdir functions/hid.usb0 2>/dev/null || true
    
    # 移除字符串
    rmdir strings/0x409 2>/dev/null || true
    
    cd ..
    rmdir "$GADGET_NAME" 2>/dev/null || true
    echo "    Old configuration cleaned"
else
    echo "[3/8] No old configuration found"
fi

# 4. 创建新的 gadget
echo "[4/8] Creating new gadget: $GADGET_NAME"
mkdir -p "$GADGET_NAME"
cd "$GADGET_NAME"

# 5. 配置 USB 设备描述符
echo "[5/8] Configuring USB device descriptors..."
echo 0x1d6b > idVendor  # Linux Foundation
echo 0x0104 > idProduct # Multifunction Composite Gadget
echo 0x0100 > bcdDevice # v1.0.0
echo 0x0200 > bcdUSB    # USB2
echo 0xEF > bDeviceClass    # Miscellaneous
echo 0x02 > bDeviceSubClass # Common Class
echo 0x01 > bDeviceProtocol # Interface Association

# 设置字符串
mkdir -p strings/0x409
echo "0123456789" > strings/0x409/serialnumber
echo "LinTx" > strings/0x409/manufacturer
echo "LinTx Composite (Network + GamePad)" > strings/0x409/product

# 6. 创建功能
echo "[6/8] Creating functions..."

# 功能 1: RNDIS 网络 (用于SSH)
echo "    - Creating RNDIS network function"
mkdir -p functions/rndis.usb0
# RNDIS 会自动配置 MAC 地址

# 功能 2: HID 手柄
echo "    - Creating HID gamepad function"
mkdir -p functions/hid.usb0
echo 0 > functions/hid.usb0/protocol
echo 0 > functions/hid.usb0/subclass
echo 5 > functions/hid.usb0/report_length

# HID Report Descriptor (4个摇杆轴 + 8个按键)
echo "    - Writing HID report descriptor"
echo "05 01 09 05 A1 01 A1 00 05 09 19 01 29 08 15 00 25 01 95 08 75 01 81 02 05 01 09 30 09 31 09 32 09 33 15 81 25 7F 75 08 95 04 81 02 C0 C0" | xxd -r -ps > functions/hid.usb0/report_desc

# 7. 创建并配置 configuration
echo "[7/8] Creating configuration..."
mkdir -p configs/c.1/strings/0x409
echo 0x80 > configs/c.1/bmAttributes
echo 250 > configs/c.1/MaxPower  # 500 mA
echo "RNDIS Network + HID GamePad" > configs/c.1/strings/0x409/configuration

# 链接函数到配置
ln -s functions/rndis.usb0 configs/c.1/
ln -s functions/hid.usb0 configs/c.1/

# 8. 启用 USB Gadget
echo "[8/8] Enabling USB Gadget..."
UDC_DEV=$(ls /sys/class/udc 2>/dev/null | head -n 1)
if [ -z "$UDC_DEV" ]; then
    echo "Error: No UDC device found"
    exit 1
fi

echo "$UDC_DEV" > UDC
echo "    UDC device: $UDC_DEV"

# 设置为 peripheral 模式（SG2002 特定）
if [ -f /sys/devices/platform/soc/4340000.usb/musb-hdrc.4.auto/mode ]; then
    echo "    Setting USB to peripheral mode"
    echo peripheral > /sys/devices/platform/soc/4340000.usb/musb-hdrc.4.auto/mode
elif [ -f /sys/devices/platform/soc/1c19000.usb/musb-hdrc.2.auto/mode ]; then
    echo "    Setting USB to peripheral mode (alt path)"
    echo peripheral > /sys/devices/platform/soc/1c19000.usb/musb-hdrc.2.auto/mode
fi

# 配置网络接口
sleep 2
echo "Configuring network interface usb0..."
if ip link show usb0 >/dev/null 2>&1; then
    ip addr add 192.168.42.1/24 dev usb0 2>/dev/null || true
    ip link set usb0 up
    echo "    usb0: 192.168.42.1"
else
    echo "    Warning: usb0 interface not found yet (will appear when PC connects)"
fi

echo ""
echo "=== USB Composite Gadget Configured Successfully! ==="
echo ""
echo "Configured functions:"
echo "  ✓ RNDIS Network: usb0 @ 192.168.42.1"
echo "  ✓ HID GamePad: /dev/hidg0"
echo ""
echo "On your PC:"
echo "  1. Network interface should appear (configure PC as 192.168.42.2)"
echo "  2. SSH: ssh root@192.168.42.1"
echo "  3. GamePad should appear as USB HID device"
echo ""
echo "Verify with:"
echo "  ls -l /dev/hidg0"
echo "  cat /sys/class/udc/*/state"
