#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$SCRIPT_DIR/board_common.sh"

TTY_DEV="/dev/tty"
if [ ! -r "$TTY_DEV" ]; then
    echo "Cannot access $TTY_DEV. Please run in an interactive SSH terminal." >&2
    exit 1
fi

cat <<'EOF_INFO'
Attach terminal key injector.

Use keys in this SSH session:
- Arrow keys or WASD: move
- Enter: open/select
- Esc or b: back
- [ / ]: switch launcher page
- q: quit injector (and send UI quit)

Press q to stop.
EOF_INFO

emit_event() {
    event="$1"
    LINTX_SOCKET_PATH="$SOCKET_PATH" "$BIN" -- ui_emit_input --event "$event" >/dev/null 2>&1 || true
}

read_char() {
    dd if="$TTY_DEV" bs=1 count=1 2>/dev/null || true
}

read_char_timeout() {
    stty -echo -icanon min 0 time "$1" <"$TTY_DEV"
    c=$(dd if="$TTY_DEV" bs=1 count=1 2>/dev/null || true)
    stty -echo -icanon min 1 time 0 <"$TTY_DEV"
    printf '%s' "$c"
}

ORIG_STTY=$(stty -g <"$TTY_DEV")
restore_tty() {
    stty "$ORIG_STTY" <"$TTY_DEV" || true
}
trap restore_tty EXIT INT TERM
stty -echo -icanon min 1 time 0 <"$TTY_DEV"

ESC=$(printf '\033')
CR=$(printf '\r')
LF=$(printf '\n')

while :; do
    ch=$(read_char)
    [ -n "$ch" ] || continue
    case "$ch" in
        q|Q)
            emit_event quit
            break
            ;;
        "$CR"|"$LF")
            emit_event open
            ;;
        '[')
            emit_event page-prev
            ;;
        ']')
            emit_event page-next
            ;;
        w|W|k|K)
            emit_event up
            ;;
        s|S|j|J)
            emit_event down
            ;;
        a|A|h|H)
            emit_event left
            ;;
        d|D|l|L)
            emit_event right
            ;;
        b|B)
            emit_event back
            ;;
        "$ESC")
            second=$(read_char_timeout 1)
            if [ -z "$second" ]; then
                emit_event back
                continue
            fi
            if [ "$second" = "[" ] || [ "$second" = "O" ]; then
                third=$(read_char_timeout 1)
                case "$third" in
                    A) emit_event up ;;
                    B) emit_event down ;;
                    C) emit_event right ;;
                    D) emit_event left ;;
                esac
            fi
            ;;
    esac
done
