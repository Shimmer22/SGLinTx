#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$SCRIPT_DIR/board_common.sh"

RF_DEV_NAME="${1:-/dev/ttyS2}"
RF_BAUDRATE="${2:-115200}"
INPUT_MODE="${3:-stm32}" # stm32 | mock
STM32_DEV_NAME="${4:-/dev/ttyS0}"
STM32_BAUDRATE="${5:-115200}"

stop_lintx
start_server

case "$INPUT_MODE" in
    mock)
        LINTX_SOCKET_PATH="$SOCKET_PATH" "$BIN" --detach -- mock_joystick \
            >"$LOG_DIR/input_mock.log" 2>&1
        ;;
    stm32)
        LINTX_SOCKET_PATH="$SOCKET_PATH" "$BIN" --detach -- stm32_serial "$STM32_DEV_NAME" --baudrate "$STM32_BAUDRATE" \
            >"$LOG_DIR/input_stm32.log" 2>&1
        ;;
    *)
        echo "Invalid INPUT_MODE: $INPUT_MODE (expected: stm32|mock)" >&2
        exit 2
        ;;
esac

LINTX_SOCKET_PATH="$SOCKET_PATH" "$BIN" --detach -- mixer \
    >"$LOG_DIR/mixer.log" 2>&1

LINTX_SOCKET_PATH="$SOCKET_PATH" "$BIN" --detach -- rf_link_service "$RF_DEV_NAME" --baudrate "$RF_BAUDRATE" \
    >"$LOG_DIR/rf_link_service.log" 2>&1

start_ui_vo
sleep 2
show_status

cat <<EOF
CRSF GUI test started.
RF UART: $RF_DEV_NAME @ $RF_BAUDRATE
Input mode: $INPUT_MODE

Open ELRS page and verify:
- root folder navigation
- folder enter/back
- Packet Rate / Telemetry Ratio / TX Power value write
- Bind command
- Bind Phrase string editor
EOF
