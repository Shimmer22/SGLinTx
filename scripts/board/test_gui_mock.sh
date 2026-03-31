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
ELRS mock GUI test started.
This script validates the ELRS mock page only.
It does not start any input source or mixer, so Control page will stay on default values.

Controls on device:
- Up/Down: select item
- Enter: open folder / enter string edit / run action
- Left/Right: adjust value or move string cursor
- Esc or PagePrev: back/cancel edit
- PageNext: refresh
EOF
