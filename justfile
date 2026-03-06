set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

host_target := "x86_64-unknown-linux-gnu"
windows_target := "x86_64-pc-windows-gnu"
riscv_target := "riscv64gc-unknown-linux-musl"
socket_path := env_var_or_default("LINTX_SOCKET_PATH", "/tmp/lintx-rpsocket")
ui_backend := "sdl"
ui_width := "800"
ui_height := "480"
ui_fps := "30"
mock_hz := "5"

_default:
    @just --list

help:
    @just --list

check:
    cargo check

check-sdl:
    cargo check --features sdl_ui

check-win:
    cargo check --target {{windows_target}}

build-riscv:
    cross build --target {{riscv_target}} --release

build-riscv-sdl:
    cross build --target {{riscv_target}} --release --features sdl_ui

board-binary:
    @echo target/{{riscv_target}}/release/LinTx

server:
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run --features sdl_ui -- --server

server-release target="x86_64-unknown-linux-gnu":
    LINTX_SOCKET_PATH="{{socket_path}}" target/{{target}}/release/LinTx --server

ui backend="sdl" width="800" height="480" fps="30":
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run --features sdl_ui -- -- ui_demo --backend {{backend}} --width {{width}} --height {{height}} --fps {{fps}}

ui-release target="x86_64-unknown-linux-gnu" backend="sdl" width="800" height="480" fps="30":
    LINTX_SOCKET_PATH="{{socket_path}}" target/{{target}}/release/LinTx -- ui_demo --backend {{backend}} --width {{width}} --height {{height}} --fps {{fps}}

mock hz="5":
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run --features sdl_ui -- --detach -- system_state_mock --hz {{hz}}

mixer:
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run --features sdl_ui -- --detach -- mixer

mock-joystick config="mock_config.toml":
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run --features sdl_ui -- --detach -- mock_joystick --config {{config}}

adc:
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run -- --detach -- adc

joydev device="/dev/input/js0":
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run --features joydev_input -- --detach -- joy_dev {{device}}

crsf device="/dev/ttyS0" baudrate="420000":
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run -- --detach -- crsf_rc_in {{device}} --baudrate {{baudrate}}

stm32 device="/dev/ttyS0" baudrate="115200":
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run -- --detach -- stm32_serial {{device}} --baudrate {{baudrate}}

elrs device="/dev/ttyUSB0" baudrate="115200":
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run -- --detach -- elrs_tx {{device}} --baudrate {{baudrate}}

demo-wsl hz="5" width="800" height="480" fps="30":
    rm -f "{{socket_path}}" || true
    trap 'jobs -p | xargs -r kill || true; rm -f "{{socket_path}}" || true' EXIT INT TERM; \
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run --features sdl_ui -- --server & \
    sleep 1; \
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run --features sdl_ui -- --detach -- system_state_mock --hz {{hz}}; \
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run --features sdl_ui -- --detach -- mixer; \
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run --features sdl_ui -- -- ui_demo --backend sdl --width {{width}} --height {{height}} --fps {{fps}}

demo-mock hz="5" width="800" height="480" fps="30":
    rm -f "{{socket_path}}" || true
    trap 'jobs -p | xargs -r kill || true; rm -f "{{socket_path}}" || true' EXIT INT TERM; \
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run --features sdl_ui -- --server & \
    sleep 1; \
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run --features sdl_ui -- --detach -- system_state_mock --hz {{hz}}; \
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run --features sdl_ui -- --detach -- mock_joystick --config mock_config.toml; \
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run --features sdl_ui -- --detach -- mixer; \
    LINTX_SOCKET_PATH="{{socket_path}}" cargo run --features sdl_ui -- -- ui_demo --backend sdl --width {{width}} --height {{height}} --fps {{fps}}

board-help:
    @echo "Build SG2002 binary:    just build-riscv"
    @echo "Binary path:            just board-binary"
    @echo "On board start server:  ./LinTx --server"
    @echo "On board start modules: ./LinTx --detach -- mixer"
    @echo "                        ./LinTx --detach -- adc"
    @echo "                        ./LinTx --detach -- crsf_rc_in /dev/ttyS0"
    @echo "                        ./LinTx --detach -- elrs_tx /dev/ttyS1"
