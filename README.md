# LinTx 项目说明

## 项目简介
LinTx 是一个基于 Rust 的模块化遥控系统应用，当前同时支持：
- 板卡目标：**RISC-V 64 位**（`riscv64gc-unknown-linux-musl`，Buildroot/BusyBox，SG2002）
- 桌面目标：**x86_64 Linux**
- 桌面目标：**x86_64 Windows**（`x86_64-pc-windows-gnu`）

## 关键特性
- 使用 **musl** 静态链接，二进制体积小，运行时依赖最少。
- 通过 **cross** + 自定义 Docker 镜像（包含 binutils 2.42）实现跨编译。
- 模块化设计，支持多种协议输入（如 STM32 串口、CRSF）和功能模块（如 ADC 读取、混控器、ELRS 发射）。

## 编译步骤
```bash
# 1. 清理旧的构建产物
cargo clean

# 2. 编译当前主机（x86_64 Linux）
cargo check

# 3. 编译 Windows 目标（首次需要安装 target）
rustup target add x86_64-pc-windows-gnu
cargo check --target x86_64-pc-windows-gnu

# 4. 使用 cross 编译 RISC-V（已配置自定义镜像）
cross build --target riscv64gc-unknown-linux-musl --release
```
编译完成后，二进制位于 `target/riscv64gc-unknown-linux-musl/release/LinTx`。

### 可选功能
- `joydev_input`：启用 Linux `joydev` 输入模块
```bash
cargo check --features joydev_input
```
- `sdl_ui`：启用 PC SDL 窗口 UI 后端（WSL2/桌面调试建议开启）
```bash
cargo check --features sdl_ui
```

## 如何使用

LinTx 采用客户端-服务器架构（基于 `rpos` 库）。主程序通常作为服务器后台运行，通过命令行参数启动具体的子模块。

### 基本用法（Linux）
```bash
# 格式
./LinTx -- <模块名称> [模块参数]
```

### 基本用法（Windows）
Windows 下使用本地模式（不依赖 Unix socket server）：
```bash
LinTx -- <模块名称> [模块参数]
```

### 可用模块及参数

#### 1. `stm32_serial` (STM32 自定义串口协议)
用于读取 STM32 发送的自定义协议遥控数据（0x5A 头）。
- **参数**:
  - `<设备路径>`: (必选，位置参数) 串口设备路径，例如 `/dev/ttyS0`。
  - `--baudrate <波特率>`: (可选) 串口波特率，默认 `115200`。
- **示例**:
  ```bash
  ./LinTx -- stm32_serial /dev/ttyS0 --baudrate 115200
  ```

#### 2. `crsf_rc_in` (CRSF 协议输入)
用于读取标准 CRSF 协议的遥控数据。
- **参数**:
  - `<设备路径>`: (必选，位置参数) 串口设备路径。
  - `--baudrate <波特率>`: (可选) 默认 `420000`。
- **示例**:
  ```bash
  ./LinTx -- crsf_rc_in /dev/ttyS0
  ```

#### 3. `elrs_tx` (ELRS 发射模块)
用于驱动 ELRS 发射高频头。
- **参数**:
  - `<设备路径>`: (必选，位置参数) 串口设备路径。
  - `--baudrate <波特率>`: (可选) 默认 `115200`。
- **示例**:
  ```bash
  ./LinTx -- elrs_tx /dev/ttyS1
  ```

#### 4. `adc` (ADC 读取)
读取 ADS1115 ADC 数据（通常用于直接读取摇杆电位器）。
- **参数**: 无（硬编码使用 `/dev/i2c-0`）。
- **示例**:
  ```bash
  ./LinTx -- adc
  ```

#### 5. `mock_joystick` (模拟摇杆数据生成器)
**用于测试目的**：无需物理硬件（不占用IIC/UART），模拟生成摇杆数据用于测试CRSF发送等功能。

- **参数**:
  - `--config <配置文件路径>`: (可选) 配置文件路径，默认 `mock_config.toml`。
- **模式**:
  - **static**: 发送固定的通道值（如居中的摇杆位置）
  - **sine**: 发送正弦波振荡的通道值（用于测试平滑过渡）
  - **step**: 发送离散的阶跃值（用于测试响应）
- **配置文件示例** (`mock_config.toml`):
  ```toml
  mode = "static"  # "static", "sine", "step"
  update_rate_hz = 50

  [static_config]
  channels = [992, 992, 0, 992]  # CRSF居中值

  [sine_config]
  base = [992, 992, 0, 992]
  amplitude = [200, 150, 0, 100]
  frequency_hz = [1.0, 0.5, 0.0, 2.0]

  [step_config]
  values = [
      [0, 0, 0, 0],
      [992, 992, 0, 992],
      [1984, 1984, 1984, 1984]
  ]
  step_duration_ms = 2000
  ```
- **示例**:
  ```bash
  # 使用默认配置
  ./LinTx -- mock_joystick
  
  # 使用自定义配置
  ./LinTx -- mock_joystick --config my_mock.toml
  
  # 配合ELRS发射模块测试CRSF发送能力
  ./LinTx --server &
  ./LinTx -- mock_joystick &
  ./LinTx -- elrs_tx /dev/ttyS1
  ```

#### 6. `mixer` (混控器)
处理输入数据并进行混控逻辑（依赖 `joystick.toml` 配置文件）。
- **参数**: 无。
- **示例**:
  ```bash
  ./LinTx -- mixer
  ```

#### 7. `usb_gamepad` (USB HID 手柄输出)
将混控后的数据输出到 USB HID 手柄设备，使从机模拟成 PC 可识别的游戏手柄。
- **前置条件**: 需先运行 `gamepad_composite.sh` 配置 USB Gadget。
- **参数**:
  - `--device <设备路径>`: (可选) HID 设备路径，默认 `/dev/hidg0`。
- **示例**:
  ```bash
  # 1. 配置 USB 复合设备（网络 + 手柄）
  sh gamepad_composite.sh
  
  # 2. 启动完整流程：mock数据 -> mixer -> USB输出
  ./LinTx --server &
  ./LinTx -- mock_joystick &
  ./LinTx -- mixer &
  ./LinTx -- usb_gamepad
  
  # 或使用真实的 STM32 串口输入
  ./LinTx -- stm32_serial /dev/ttyS0 &
  ./LinTx -- mixer &
  ./LinTx -- usb_gamepad
  ```
- **通道映射** (mixer输出 → HID轴):
  - `thrust` (油门) → HID Rz轴
  - `direction` (方向) → HID X轴
  - `aileron` (副翼) → HID Z轴
  - `elevator` (升降) → HID Y轴
  - mixer 值域: 0~10000 (中心值 5000)
  - HID 值域: -127~127 (中心值 0)

#### 8. `system_state_mock` (系统状态/配置模拟源)
用于向 UI 发送基础系统数据：
- 遥控电量
- 飞行器电量
- 信号强度
- 系统时间
- 背光、声音配置

示例：
```bash
./LinTx -- system_state_mock --hz 5
```

#### 9. `ui_demo` (LVGL 框架基础应用)
这是新的 UI 框架入口（当前用终端后端演示，接口已按 LVGL 架构抽象）：

- `--backend sdl`：PC SDL 窗口后端（支持 `--width/--height`）
- `--backend pc`：PC 终端后端
- `--backend fb --fb-device /dev/fb0`：板卡 framebuffer 后端（后续可接 MIPI 屏）

示例：
```bash
# Linux: server/client 方式
./LinTx --server &
./LinTx -- system_state_mock --hz 5 &
./LinTx -- ui_demo --backend sdl --width 800 --height 480 --fps 30

# 板卡场景（示例）
./LinTx --server &
./LinTx -- system_state_mock --hz 5 &
./LinTx -- ui_demo --backend fb --fb-device /dev/fb0
```

WSL2 测试建议：
```bash
cargo run --features sdl_ui -- --server
cargo run --features sdl_ui -- -- system_state_mock --hz 5
cargo run --features sdl_ui -- -- ui_demo --backend sdl --width 800 --height 480 --fps 30
```

## LVGL 架构设计（当前已落地基础骨架）
当前代码新增 `src/ui/` 分层，便于与现有 `rpos` 架构融合并支持扩展：

- `ui/backend.rs`
  - `LvglBackend` trait：统一 PC 与 fb 后端接口
  - `BackendKind::PcApi | Fbdev`
- `ui/model.rs`
  - `UiFrame`：统一 UI 数据模型（状态页 + 配置页）
- `ui/app.rs`
  - 主循环：订阅消息、切屏、渲染
- `messages.rs`
  - 统一消息定义：`adc_raw`、`system_status`、`system_config`

后续接入真实 LVGL 时建议：
- PC 端：在 `PcApi` 后端接 `lvgl + SDL/Win32` 刷新
- SG2002 板卡：在 `Fbdev` 后端接 `/dev/fb0`，显示驱动走 MIPI/fb
- 业务模块通过 `rpos::msg` 持续推送状态和配置，UI 只消费消息，不直接耦合驱动

## 许可证
本项目遵循 `MIT` 许可证（详见 `LICENSE` 文件）。
