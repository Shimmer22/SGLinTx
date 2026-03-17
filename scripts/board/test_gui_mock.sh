#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$SCRIPT_DIR/board_common.sh"

stop_lintx
start_server

LINTX_SOCKET_PATH="$SOCKET_PATH" "$BIN" --detach -- elrs_agent --mode mock --dev-name /dev/ttyS3 \
    >"$LOG_DIR/elrs_mock.log" 2>&1

start_ui_fb
sleep 2
show_status

cat <<'EOF'
Mock GUI test started.
Controls on device:
- Up/Down: select item
- Enter: open folder / enter string edit / run action
- Left/Right: adjust value or move string cursor
- Esc or PagePrev: back/cancel edit
- PageNext: refresh
EOF
