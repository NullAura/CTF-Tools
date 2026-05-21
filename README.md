# CTF Tools

CTF Tools 是一个面向 CTF 训练、比赛解题和本地安全实验的桌面工具箱。项目使用 Rust 构建核心执行链路和高频算法，使用 Python worker 承载适合脚本化扩展的能力，目标是在保持响应速度的同时，让编码、Web、Crypto、取证、Pwn、逆向和题目整理工作集中到一个统一工作台里。

桌面端默认采用英文界面，并支持在设置中切换中文。所有核心功能通过统一的 Operation 注册表暴露，CLI 和 GUI 共用同一套执行链路，避免功能重复实现。

## 功能概览

- **CyberChef 式工具链**：在 Operations 工作台中搜索工具、添加到 Recipe、按步骤叠加运行编码/解码/转换流程。
- **编码与转换**：Base64、Base32、Base45、Base58、Base62、Base85、URL、HTML、Unicode、ASCII、Hex、二进制、八进制、十进制、hexdump、端序转换、摩斯、Brainfuck、自动解码。
- **哈希与密码**：MD5、SHA1、SHA2、SHA3、NTLM、SM3、ROT13、Caesar、XOR 单字节爆破。
- **Web / HTTP / JWT**：raw HTTP request 解析，生成 Python `requests` / `httpx`、curl、fetch 代码，JWT decode、HS256 sign/verify、弱密钥测试。
- **资产分拣**：从混合文本中提取 URL、域名、IP、C 段、邮箱、手机号、身份证格式等目标，并输出分组结果。
- **文件、隐写与流量分析**：Hex Viewer、熵分析、图片 Data URI、GIF 分帧、PCAP HTTP/DNS/ICMP/TCP 摘要、USB HID 按键解析。
- **Pwn 与逆向**：ELF/PE/Mach-O 基础信息、strings、cyclic pattern、pack/unpack、x86_64 shellcode asm/disasm。
- **本地工具启动器**：管理 Python、Java、Shell、GUI 应用和 URL 工具，支持收藏、最近使用、环境扫描和路径复制。
- **题目工作台**：提供题目记录和 writeup 模板入口，方便沉淀解题过程。

## 桌面端

启动 GUI：

```bash
cargo run -p ctf-app
```

Operations 工作台适合高频解题操作：

1. 在左侧分类或搜索框中找到工具。
2. 双击或添加到 Recipe。
3. 在右侧输入文本、bytes 或文件路径。
4. 点击 Run recipe 查看结果、trace 和 warning。

Launcher 工作台用于管理外部工具：

1. 添加本地脚本、JAR、二进制、GUI 应用或 URL。
2. 配置默认 Python / Java 环境。
3. 使用收藏、最近使用和搜索快速启动工具。

## CLI

列出工具：

```bash
cargo run -p ctf-cli -- list
```

搜索工具：

```bash
cargo run -p ctf-cli -- search base64
```

运行 Operation：

```bash
cargo run -p ctf-cli -- run base64.decode --text "ZmxhZ3t0ZXN0fQ=="
```

Launcher 命令：

```bash
cargo run -p ctf-cli -- launcher list
cargo run -p ctf-cli -- launcher search cyber
cargo run -p ctf-cli -- launcher scan-envs
```

## 项目结构

```text
crates/
  ctf-app       桌面端 GUI，基于 eframe/egui
  ctf-cli       命令行入口
  ctf-core      Operation schema、注册表、Runner 基础类型
  ctf-runner    Rust handler 与 Python worker 调度
  ctf-codecs    编码、转换、自动解码
  ctf-crypto    哈希、古典密码、口令相关能力
  ctf-web       HTTP、JWT、资产分拣
  ctf-stego     文件、图片、PCAP、USB HID 分析
  ctf-pwn       Pwn 与逆向基础工具
  ctf-launcher  本地工具启动器数据模型与环境管理
registry/
  operations.toml  内置 Operation 注册表
python/
  worker 与 Python 扩展测试
docs/
  开发文档、插件模型和阶段计划
```

## 安全边界

- 默认功能以本地解析、转换、分析为主。
- 主动扫描、请求重放、空间测绘 API 查询等高风险能力默认不启用。
- HTTP 代码生成会对敏感 Header / Cookie 做默认脱敏。
- 长任务通过统一 runner 施加超时和输出大小限制。
- Python worker 使用 JSON Lines 协议，便于隔离、恢复和插件化扩展。

## 开发与测试

常用门禁：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
PYTHONPATH=python .venv/bin/python -m pytest python/tests
```

文档入口：

- [开发文档](./docs/DEVELOPMENT.md)
- [插件与适配器模型](./docs/PLUGIN_MODEL.md)
- [阶段计划](./docs/MVP_PLAN.md)
