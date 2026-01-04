#!/bin/sh
# 安全地添加 HID 功能到 g0（即使 SSH 断开也能完成）

# 这个脚本会在后台运行，等待几秒后自动恢复网络

add_hid_function() {
    G0_DIR="/sys/kernel/config/usb_gadget/g0"
    
    if [ ! -d "$G0_DIR" ]; then
        echo "Error: g0 not found" > /tmp/hid_add.log
        return 1
    fi
    
    cd "$G0_DIR"
    
    # 记录当前 UDC
    CURRENT_UDC=$(cat UDC 2>/dev/null)
    echo "Current UDC: $CURRENT_UDC" > /tmp/hid_add.log
    
    # 1. 禁用 gadget
    echo "Disabling gadget..." >> /tmp/hid_add.log
    echo "" > UDC
    sleep 2
    
    # 2. 创建 HID 功能
    echo "Creating HID function..." >> /tmp/hid_add.log
    if [ ! -d "functions/hid.usb0" ]; then
        mkdir -p functions/hid.usb0
        echo 0 > functions/hid.usb0/protocol
        echo 0 > functions/hid.usb0/subclass
        echo 5 > functions/hid.usb0/report_length
        echo "05 01 09 05 A1 01 A1 00 05 09 19 01 29 08 15 00 25 01 95 08 75 01 81 02 05 01 09 30 09 31 09 32 09 33 15 81 25 7F 75 08 95 04 81 02 C0 C0" | xxd -r -ps > functions/hid.usb0/report_desc
        echo "HID function created" >> /tmp/hid_add.log
    else
        echo "HID function already exists" >> /tmp/hid_add.log
    fi
    
    # 3. 链接到配置
    echo "Linking HID to config..." >> /tmp/hid_add.log
    CONFIG_DIR=$(ls -1d configs/c.* 2>/dev/null | head -n 1)
    if [ -n "$CONFIG_DIR" ] && [ ! -L "$CONFIG_DIR/hid.usb0" ]; then
        ln -s functions/hid.usb0 "$CONFIG_DIR/"
        echo "HID linked" >> /tmp/hid_add.log
    fi
    
    # 4. 重新启用 gadget
    echo "Re-enabling gadget..." >> /tmp/hid_add.log
    echo "$CURRENT_UDC" > UDC
    sleep 2
    
    # 5. 验证
    if [ -c /dev/hidg0 ]; then
        echo "SUCCESS: /dev/hidg0 created" >> /tmp/hid_add.log
    else
        echo "WARNING: /dev/hidg0 not found" >> /tmp/hid_add.log
    fi
    
    echo "Done!" >> /tmp/hid_add.log
}

# 清除旧日志
rm -f /tmp/hid_add.log

echo "=== Adding HID to g0 (background mode) ==="
echo ""
echo "⚠️  WARNING: This will temporarily disconnect USB network!"
echo "    Your SSH connection will be lost for a few seconds."
echo ""
echo "The script will run in background and complete automatically."
echo "After ~5 seconds, reconnect SSH and check:"
echo "  cat /tmp/hid_add.log"
echo "  ls -l /dev/hidg0"
echo ""
echo "Press Ctrl+C within 3 seconds to cancel..."
sleep 3

# 在后台运行，即使 SSH 断开也继续
(add_hid_function) &

echo ""
echo "Background job started. SSH may disconnect now..."
echo "Wait 10 seconds, then reconnect and check /tmp/hid_add.log"
