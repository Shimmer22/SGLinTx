#!/bin/sh
set -eu

BOARD_HOST="${BOARD_HOST:-root@10.85.36.1}"
BOARD_PASSWORD="${BOARD_PASSWORD:-root}"
BOARD_DIR="${BOARD_DIR:-/root/lintx}"
TARGET_BIN="${TARGET_BIN:-target/riscv64gc-unknown-linux-musl/release/LinTx}"

if [ ! -x "$TARGET_BIN" ]; then
    echo "missing binary: $TARGET_BIN" >&2
    echo "build first: cross build --target riscv64gc-unknown-linux-musl --release" >&2
    exit 1
fi

sshpass -p "$BOARD_PASSWORD" ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null "$BOARD_HOST" \
    "mkdir -p '$BOARD_DIR/scripts/board'"

sshpass -p "$BOARD_PASSWORD" ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null "$BOARD_HOST" \
    "cat > '$BOARD_DIR/LinTx'" < "$TARGET_BIN"
sshpass -p "$BOARD_PASSWORD" ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null "$BOARD_HOST" \
    "chmod +x '$BOARD_DIR/LinTx'"

for script in scripts/board/board_common.sh \
    scripts/board/test_gui_mock.sh \
    scripts/board/test_gui_crsf.sh \
    scripts/board/test_gui_crsf_debug.sh \
    scripts/board/stop_lintx.sh
do
    base=$(basename "$script")
    target="$BOARD_DIR/scripts/board/$base"
    sshpass -p "$BOARD_PASSWORD" ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null "$BOARD_HOST" \
        "cat > '$target'" < "$script"
    sshpass -p "$BOARD_PASSWORD" ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null "$BOARD_HOST" \
        "chmod +x '$target'"
done

echo "deployed to $BOARD_HOST:$BOARD_DIR"
