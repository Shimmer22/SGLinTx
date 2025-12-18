# LinTx 项目说明

## 项目简介
LinTx 是一个基于 Rust 的嵌入式应用，目标平台为 **RISC-V 64 位**（`riscv64gc-unknown-linux-musl`），适配 **Buildroot / BusyBox** 环境，可在 **SG2002**（C906 核心）芯片上运行。

## 关键特性
- 使用 **musl** 静态链接，二进制体积小，运行时依赖最少。
- 通过 **cross** + 自定义 Docker 镜像（包含 binutils 2.42）实现跨编译。
- 已在 SG2002 上验证可执行，支持基本的串口、GPIO、ADC 等外设（通过 `rpos` 库）。

## 编译步骤
```bash
# 1. 清理旧的构建产物
cargo clean

# 2. 使用 cross 编译（已配置自定义镜像）
cross build --target riscv64gc-unknown-linux-musl --release
```
编译完成后，二进制位于 `target/riscv64gc-unknown-linux-musl/release/LinTx`。

## 部署到 SG2002
```bash
# 通过 base64 方式传输（避免 sftp-server 缺失的问题）
base64 -w 0 target/riscv64gc-unknown-linux-musl/release/LinTx \
  | sshpass -p milkv ssh -o StrictHostKeyChecking=no root@192.168.0.101 "base64 -d > /tmp/LinTx && chmod +x /tmp/LinTx"

# 运行验证
ssh root@192.168.0.101 "/tmp/LinTx --version"
```
> **注意**：如果板子上缺少 `sftp-server`，可以使用上述 `base64` 方法进行文件传输。

## 清理工作
- 已删除所有编译日志文件 (`build_error*.log`)。
- 已删除临时/奇怪文件 `root@192.`。
- `target` 目录仅保留 `release` 子目录和最终二进制，其他调试产物已清理。

## 许可证
本项目遵循 `MIT` 许可证（详见 `LICENSE` 文件）。
