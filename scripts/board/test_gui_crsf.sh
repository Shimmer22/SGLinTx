#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$SCRIPT_DIR/board_common.sh"

DEV_NAME="${1:-/dev/ttyS3}"
BAUDRATE="${2:-420000}"

stop_lintx
start_server

LINTX_SOCKET_PATH="$SOCKET_PATH" "$BIN" --detach -- elrs_agent --mode crsf --dev-name "$DEV_NAME" --baudrate "$BAUDRATE" \
    >"$LOG_DIR/elrs_crsf.log" 2>&1

start_ui_fb
sleep 2
show_status

cat <<EOF
CRSF GUI test started.
UART: $DEV_NAME
Baudrate: $BAUDRATE
Open ELRS page and verify:
- root folder navigation
- folder enter/back
- Packet Rate / Telemetry Ratio / TX Power value write
- Bind command
- Bind Phrase string editor
EOF
