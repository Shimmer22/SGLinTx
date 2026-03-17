#!/bin/sh
set -eu

APP_DIR="${APP_DIR:-/root/lintx}"
BIN="${BIN:-$APP_DIR/LinTx}"
SOCKET_PATH="${LINTX_SOCKET_PATH:-/tmp/lintx-rpsocket}"
LOG_DIR="${LOG_DIR:-/tmp/lintx-elrs}"
UI_WIDTH="800"
UI_HEIGHT="480"
FB_ROTATE="270"
FB_SWAP_RB="1"

mkdir -p "$LOG_DIR"

stop_lintx() {
    ps | awk '/LinTx/ && !/awk/ {print $1}' | while read -r pid; do
        kill "$pid" 2>/dev/null || true
    done
    rm -f "$SOCKET_PATH"
}

start_server() {
    LINTX_SOCKET_PATH="$SOCKET_PATH" \
    LINTX_FB_ROTATE="$FB_ROTATE" \
    LINTX_FB_SWAP_RB="$FB_SWAP_RB" \
    LINTX_ELRS_DEBUG="${LINTX_ELRS_DEBUG:-}" \
    "$BIN" --server >"$LOG_DIR/server.log" 2>&1 &
    sleep 1
}

start_ui_fb() {
    LINTX_SOCKET_PATH="$SOCKET_PATH" \
    LINTX_FB_ROTATE="$FB_ROTATE" \
    LINTX_FB_SWAP_RB="$FB_SWAP_RB" \
    "$BIN" -- ui_demo --backend fb --fb-device /dev/fb0 --width "$UI_WIDTH" --height "$UI_HEIGHT" \
    >"$LOG_DIR/ui.log" 2>&1 &
}

show_status() {
    echo "== LinTx processes =="
    ps | grep LinTx | grep -v grep || true
    echo
    echo "== Logs =="
    for log in "$LOG_DIR"/*.log; do
        [ -f "$log" ] || continue
        echo "-- $log --"
        tail -n 20 "$log" || true
    done
    echo
    echo "UI_WIDTH=$UI_WIDTH UI_HEIGHT=$UI_HEIGHT FB_ROTATE=$FB_ROTATE FB_SWAP_RB=$FB_SWAP_RB"
}
