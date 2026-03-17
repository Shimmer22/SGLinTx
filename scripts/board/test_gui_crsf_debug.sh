#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$SCRIPT_DIR/board_common.sh"

DEV_NAME="${1:-/dev/ttyS3}"
BAUDRATE="${2:-420000}"

export LINTX_ELRS_DEBUG=1

stop_lintx
start_server

LINTX_SOCKET_PATH="$SOCKET_PATH" \
    "$BIN" --detach -- elrs_agent --mode crsf --dev-name "$DEV_NAME" --baudrate "$BAUDRATE" \
    >"$LOG_DIR/elrs_crsf_debug.log" 2>&1

start_ui_fb
sleep 2
show_status

echo "Debug mode enabled. Inspect $LOG_DIR/elrs_crsf_debug.log for CRSF hex traffic."
