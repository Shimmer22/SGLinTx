# ELRS / CRSF 配置任务交接

本文件给下一位 AI agent 使用。
目标不是泛化维护整个仓库，而是继续推进“让这个 Linux TX 像 EdgeTX 遥控器一样，具备较完整的 ELRS 配置能力”。

## 当前目标

让 `LinTx` 通过 `UART3 + CRSF` 与 ELRS 模块交互，逐步实现接近 EdgeTX + ExpressLRS.lua 的体验，包括：

- 发现 ELRS 模块
- 读取并展示 ELRS 参数树
- 修改参数
- 执行动作类命令
- 支持 `bind phrase` 等字符串配置项
- 在 UI 中可操作，而不是只靠命令行或脚本

## 当前已完成

### 1. Lua 基础运行时

已加入最小 Lua 支持：

- 文件: [src/lua_run.rs](/home/shimmer/LinTx/LinTx_musl/src/lua_run.rs)
- 功能:
  - `uart_open(path, baudrate)`
  - 原始串口读写
  - 十六进制收发
  - `crsf.encode(...)`
  - `crsf.rc_channels(...)`

说明：
- 这是仓内脚本能力，不是直接兼容 EdgeTX Lua API。
- 后续如有必要，可以继续把 CRSF 参数操作暴露给 Lua。

### 2. ELRS 状态/命令消息总线

已新增 ELRS 相关消息：

- 文件: [src/messages.rs](/home/shimmer/LinTx/LinTx_musl/src/messages.rs)
- 结构:
  - `ElrsStateMsg`
  - `ElrsParamEntry`
  - `ElrsCommandMsg`

作用：
- UI、协议后端、将来可能的 Lua/脚本层之间已经解耦。

### 3. ELRS UI 页面

现有 UI 的 `Scripts` 页面已改造成 ELRS 页面：

- 文件:
  - [src/ui/app.rs](/home/shimmer/LinTx/LinTx_musl/src/ui/app.rs)
  - [src/ui/backend.rs](/home/shimmer/LinTx/LinTx_musl/src/ui/backend.rs)
  - [src/ui/catalog.rs](/home/shimmer/LinTx/LinTx_musl/src/ui/catalog.rs)
  - [src/ui/model.rs](/home/shimmer/LinTx/LinTx_musl/src/ui/model.rs)

当前能力：
- 展示连接状态
- 展示模块名 / 设备名 / 版本 / 状态文本
- 展示参数列表
- UI 发出 ELRS 命令:
  - 上下选择
  - 左右改值
  - 回车执行

### 4. `elrs_agent` 已有 mock + 真实 `crsf` 后端

- 文件: [src/elrs_agent.rs](/home/shimmer/LinTx/LinTx_musl/src/elrs_agent.rs)

当前支持：

- `--mode mock`
  - 无硬件演示 UI 行为

- `--mode crsf`
  - 打开串口
  - CRSF 分帧 / CRC 校验
  - 发 `PING_DEVICES`
  - 解析 `DEVICE_INFO`
  - 解析 `ELRS info`
  - 发 bind 类命令
  - 发参数读取帧 `0x2C`
  - 解析参数返回帧 `0x2B`
  - 支持分块参数拼接
  - 支持基础字段类型:
    - `UINT8`
    - `TEXT_SELECTION`
    - `STRING`
    - `FOLDER`
    - `INFO`
    - `COMMAND`
  - 支持基础参数写回帧 `0x2D`

### 5. 文档已更新

- 文件: [README.md](/home/shimmer/LinTx/LinTx_musl/README.md)

已写入：
- `lua` feature
- `lua_run`
- `elrs_agent --mode mock`
- `elrs_agent --mode crsf`

## 当前真实状态

### 已经“能用”的部分

如果板上串口连好，以下能力理论上已经具备：

- 发现 ELRS 模块
- 显示 ELRS 页面
- 读取一部分真实参数字段
- 对一部分基础字段尝试写回
- 执行动作类字段

### 还没有完成的关键点

这部分是下一位 agent 的重点。

#### A. 还没有真正的“参数树导航”

当前不是完整目录树浏览，而是：

- 扫描前一段字段 ID
- 把解析到的字段平铺显示到 UI

问题：
- 没有 folder 进入/返回
- 没有 parent/child 导航状态
- 还不等价于 EdgeTX 里的完整 ELRS 菜单

#### B. 字段发现仍然是保守实现

当前 `pending_reads` 初始是 `1..=20`。

问题：
- 这只是联调用的保守扫描范围
- 很可能看不到全部 ELRS 参数
- 某些模块、版本、布局不同，字段 ID 不一定都落在这个范围

#### C. `bind phrase` 还没有打通

这是任务重点之一。

原因：
- `bind phrase` 大概率会以 `STRING` 字段暴露
- 当前虽然能解析 `STRING`
- 但 UI 还没有字符串编辑器
- 也没有针对字段写入后的确认/刷新流程

结论：
- 现在还不能宣称“支持配置绑定词”

#### D. `STRING` 字段还不能编辑

这是 `bind phrase` 的直接阻塞项。

#### E. `FOLDER` 只是识别了类型，还不能进入

需要做：
- 当前 folder 作为列表项显示
- 回车进入 child 列表
- 支持返回上级

#### F. 还没有真实硬件抓包/联调校验结果

当前情况：
- 编译通过
- 协议实现参考了 EdgeTX 的 `crossfire.cpp` 与 `crossfire-parse.py`
- 但没有记录“某个真实 ELRS 模块已完整实测通过”的结果

这意味着下一位 agent 不要假设协议实现已经完全正确。
需要准备真实串口日志，必要时增加调试输出。

## 参考来源

开发时已经参考过：

- `../EdgeTX_ref/radio/src/pulses/crossfire.cpp`
- `../EdgeTX_ref/radio/src/telemetry/crossfire.cpp`
- `../EdgeTX_ref/radio/util/crossfire-parse.py`

关键点：

- 普通 CRSF 帧: CRC8 DVB-S2
- `COMMAND_ID` 扩展命令: 还有一层 CRC8_BA
- 参数相关帧:
  - `0x2B` parameter entry
  - `0x2C` parameter read/request
  - `0x2D` parameter write/update
  - `0x2E` ELRS info

## 下一位 agent 的首要任务

按优先级执行，不要分散到别的功能。

### 1. 把“平铺字段”升级成“参数树”

目标：
- 不再只是扫 `1..=20`
- 根据 folder / parent / child 关系构建树
- UI 支持进入目录和返回

建议做法：
- 在 `src/elrs_agent.rs` 中增加字段缓存结构:
  - `field_id -> field`
  - `parent -> children`
- 在 UI 中维护当前 folder id / path
- 只显示当前 folder 下的字段

交付标准：
- 页面行为更像遥控器菜单，而不是调试列表

### 2. 支持 `STRING` 字段编辑

这是当前最重要的功能缺口。

目标：
- 让 `bind phrase` 能真正可配

建议做法：
- 先实现一个最简编辑模式
  - 选择字符串字段后进入编辑
  - 使用已有输入事件做字符选择/移动/确认
- 写回协议必须谨慎
  - 先确认 `0x2D` 对字符串字段的实际格式
  - 不要凭空假设只写一个字节

注意：
- 当前 `build_param_write_frame(field_id, value: u8)` 只适合基础单字节字段
- 对字符串字段必须扩展 payload 结构

### 3. 增加真实串口调试日志

目标：
- 能验证 ELRS 模块实际返回了什么
- 能在出现兼容问题时快速定位

建议：
- 仅在 debug 开关下打印
- 输出内容:
  - 原始 CRSF 帧 hex
  - 参数读请求 / 返回
  - 分块重组过程
  - 字段解析失败原因

不要默认刷屏。

### 4. 用真实模块联调

优先验证这些字段是否被正确识别：

- Packet Rate
- Telemetry Ratio
- TX Power
- WiFi Update
- Bind
- Bind Phrase

如果字段名称或类型与当前假设不一致，优先以真实返回内容修正实现。

## 不要做的事

- 不要把注意力转到无关模块
- 不要为了“看起来完整”重写整个 UI
- 不要假设 EdgeTX 的所有行为都能直接照搬
- 不要在未确认协议格式前乱写字符串字段
- 不要破坏现有 `mock` 模式，保留它作为无硬件联调入口

## 当前启动方式

### 无硬件 mock

```bash
./LinTx --server &
./LinTx -- elrs_agent --mode mock --dev-name /dev/ttyS3 &
./LinTx -- ui_demo --backend sdl --width 800 --height 480 --fps 30
```

### 真实 CRSF / ELRS

```bash
./LinTx --server &
./LinTx -- elrs_agent --mode crsf --dev-name /dev/ttyS3 --baudrate 420000 &
./LinTx -- ui_demo --backend fb --fb-device /dev/fb0
```

## 当前验证情况

已完成：

- `cargo fmt`
- `cargo check --features lua`

未完成：

- 真实板卡 / 真实 ELRS 模块行为验收
- 参数树完整验证
- `bind phrase` 实测写入

## 最后一句

下一位 agent 的任务重点只有一句话：

把当前“能探测模块、能读部分参数的 ELRS 页面”，推进成“能像遥控器一样浏览真实 ELRS 参数树，并能编辑 bind phrase”的版本。
