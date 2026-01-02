# LinTx 项目说明

## 项目简介
LinTx 是一个基于 Rust 的嵌入式应用，目标平台为 **RISC-V 64 位**（`riscv64gc-unknown-linux-musl`），适配 **Buildroot / BusyBox** 环境，可在 **SG2002**（C906 核心）芯片上运行。

## 关键特性
- 使用 **musl** 静态链接，二进制体积小，运行时依赖最少。
- 通过 **cross** + 自定义 Docker 镜像（包含 binutils 2.42）实现跨编译。
- 模块化设计，支持多种协议输入（如 STM32 串口、CRSF）和功能模块（如 ADC 读取、混控器、ELRS 发射）。

## 编译步骤
```bash
# 1. 清理旧的构建产物
cargo clean

# 2. 使用 cross 编译（已配置自定义镜像）
cross build --target riscv64gc-unknown-linux-musl --release
```
编译完成后，二进制位于 `target/riscv64gc-unknown-linux-musl/release/LinTx`。

## 如何使用

LinTx 采用客户端-服务器架构（基于 `rpos` 库）。主程序通常作为服务器后台运行，通过命令行参数启动具体的子模块。

### 基本用法
```bash
# 格式
./LinTx -- <模块名称> [模块参数]
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

#### 5. `mixer` (混控器)
处理输入数据并进行混控逻辑（依赖 `joystick.toml` 配置文件）。
- **参数**: 无。
- **示例**:
  ```bash
  ./LinTx -- mixer
  ```

## 许可证
本项目遵循 `MIT` 许可证（详见 `LICENSE` 文件）。
