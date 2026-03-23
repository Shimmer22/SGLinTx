#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$SCRIPT_DIR/board_common.sh"

export LINTX_UI_DEBUG=1
export LINTX_TOUCH_DEBUG=1

stop_lintx
start_server

LINTX_SOCKET_PATH="$SOCKET_PATH" "$BIN" --detach -- elrs_agent --mode mock --dev-name /dev/ttyS3 \
    >"$LOG_DIR/elrs_mock.log" 2>&1

start_ui_fb
sleep 2
show_status

cat <<'EOF'
Mock GUI touch debug started.
Touch debug logs:
- /tmp/lintx-elrs/server.log
- /tmp/lintx-elrs/ui.log

Suggested commands on board:
- tail -f /tmp/lintx-elrs/server.log
- grep 'touch\\|evdev' /tmp/lintx-elrs/server.log
EOF
