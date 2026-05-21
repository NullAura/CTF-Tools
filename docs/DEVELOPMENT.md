# CTF 工具箱开发文档

文档版本：0.1  
日期：2026-05-21  
目标技术栈：Rust + Python  
目标形态：跨平台离线桌面工具 + 命令行工具 + 可扩展插件系统

## 1. 项目目标

本项目要开发一个面向 CTF、密码学练习、编码解码、隐写分析和常见 Web 辅助任务的工具箱。功能覆盖以公开资料中列出的“随波逐流 CTF 编码工具 v7.2/v7.21”能力为基线，包括编码解码、古典密码、中文趣味编码、带 key 与多 key 加解密、进制转换、哈希与爆破、文件图片隐写、密文编辑、Web 辅助、右键快捷操作和一键解码等模块。

实现方式采用净室重写：只把公开功能名称和通用算法规范作为需求输入，不复制原软件代码、界面、资源文件或私有实现。

核心目标：

- 离线优先：常用编码、解码、分析、隐写功能不依赖网络。
- 高性能：高频文本处理、二进制处理、哈希、XOR、批量文件扫描放在 Rust 中实现。
- 易扩展：用统一的 `Operation` 注册表承载算法、参数、输入输出类型和 UI 元数据。
- 可脚本化：所有核心能力同时暴露给 CLI，便于比赛时管道化处理。
- 安全可控：Web 扫描、第三方命令、Python/JS/Java/PHP 运行器默认受限、可超时、可审计。

## 2. 范围与非目标

### 2.1 第一阶段范围

第一阶段只做“可用且可扩展”的主干：

- 文本输入、文件输入、图片输入三类入口。
- 编码解码、ROT、进制转换、哈希、XOR、常见古典密码。
- 文件读取为 hex/bin/oct、Base64 文件互转、图片基础处理。
- 操作搜索、收藏、历史记录、批处理队列。
- CLI 与桌面 GUI 共用同一套 Rust 核心。

### 2.2 后续阶段范围

后续逐步补齐：

- 复杂隐写、音频分析、二维码修复、压缩包修复和爆破。
- 一键解码的启发式识别与评分。
- Python、JavaScript、Java、PHP 代码运行器。
- Web 辅助模块，包括 GET 查看、robots 查看、目录扫描、端口扫描、轻量 SQLi 探测、HTTP request 数据包转代码。
- 实战增强模块，包括 JWT/Session、RSA 攻击辅助、PCAP 流量取证、逆向/Pwn 辅助、字典生成、题目工作台。
- 在线解密适配器，作为可选插件，不作为核心离线能力。

### 2.3 非目标

- 不提供未授权攻击、持久化控制、凭据窃取、漏洞利用链自动化。
- 不反编译、复刻或分发闭源工具的代码与资源。
- 不把所有功能硬编码到 UI。所有工具必须从注册表加载，便于测试、搜索和 CLI 复用。

## 3. 技术选型

### 3.1 Rust

Rust 负责性能关键路径和主程序：

- GUI：`eframe/egui`，保持 Rust + Python 主栈，不额外引入前端运行时。
- CLI：`clap`。
- 异步与任务：`tokio`，长任务通过取消令牌和进度事件反馈。
- 并行：`rayon` 用于批量转换、哈希、文件扫描。
- 加密与哈希：优先使用 RustCrypto 生态。
- 二进制处理：`memmap2`、流式 reader、零拷贝切片。
- 图片基础处理：`image`、`rqrr`、`bardecoder` 或等价 Rust 库，复杂处理交给 Python。
- 压缩包：`zip`、`sevenz-rust`、外部工具适配器。

### 3.2 Python

Python 负责生态成熟但不适合在 Rust 中重复实现的能力：

- 图片隐写与图像分析：`Pillow`、`numpy`、`opencv-python`。
- 音频分析：`scipy`、`numpy`、`soundfile`。
- 第三方 CTF 工具包装：`zsteg`、`outguess`、`binwalk`、`foremost`、`steghide` 等通过适配器调用。
- 快速新增算法原型：先用 Python 插件验证，再按性能需要迁移到 Rust。

Python 不参与逐字符热循环。Rust 与 Python 之间以批量请求通信，避免频繁跨语言调用。

### 3.3 进程边界

默认使用 Python worker 子进程，而不是直接内嵌解释器：

- Rust 主进程通过 JSON Lines 或 MessagePack 与 Python worker 通信。
- 每个任务有超时、工作目录、输入文件白名单和最大输出大小。
- worker 崩溃不会带崩 GUI。
- 发布版可按平台打包独立 Python 环境；开发版使用 `uv` 管理依赖。

## 4. 总体架构

```text
ctf-tools/
  crates/
    ctf-core/        # Operation、参数、错误、任务、注册表
    ctf-codecs/      # base、rot、编码转换、进制转换
    ctf-crypto/      # 古典密码、现代密码、哈希、HMAC
    ctf-stego/       # 文件/图片/音频隐写的 Rust 部分
    ctf-web/         # Web 辅助、扫描器、HTTP 客户端
    ctf-runner/      # Python/JS/Java/PHP/外部命令运行器
    ctf-cli/         # 命令行入口
    ctf-app/         # egui 桌面应用
  python/
    ctf_toolbox_worker/
      plugins/       # Python 插件
      adapters/      # 第三方工具适配器
  registry/
    operations.toml  # 全部功能注册表
    aliases.toml     # 搜索别名、中文名、英文名、常见拼写
  tests/
    fixtures/        # 样本、图片、压缩包、音频、golden vectors
  docs/
```

配套工程文档：

- [MVP 实施计划](./MVP_PLAN.md)：第一版交付边界、开发顺序和验收标准。
- [插件与适配器模型](./PLUGIN_MODEL.md)：Python worker、外部命令、模板包和权限模型。
- [TscanPlus 对标记录](./TSCANPLUS_REFERENCE.md)：本机 TscanPlus 可见功能的产品对标和安全取舍。
- [Operation 注册表草案](../registry/operations.toml)：CLI 与 GUI 的机器可读功能清单。

核心数据流：

```text
输入文本/文件/图片
  -> InputNormalizer
  -> OperationRegistry 搜索或自动识别
  -> Rust Operation 或 Python Adapter
  -> OutputArtifact
  -> 结果面板、文件导出、历史记录、CLI stdout
```

## 5. Operation 设计

所有工具统一抽象为 `Operation`。

```rust
pub struct OperationSpec {
    pub id: OperationId,
    pub name_zh: String,
    pub name_en: String,
    pub category: Category,
    pub aliases: Vec<String>,
    pub input: Vec<InputKind>,
    pub output: Vec<OutputKind>,
    pub params: Vec<ParamSpec>,
    pub backend: BackendKind,
    pub safety: SafetyLevel,
    pub deterministic: bool,
    pub batchable: bool,
}
```

执行请求示例：

```json
{
  "operation": "base64.decode",
  "input": {"kind": "text", "value": "ZmxhZ3t0ZXN0fQ=="},
  "params": {"strict": false},
  "limits": {"timeout_ms": 3000, "max_output_bytes": 10485760}
}
```

执行结果示例：

```json
{
  "status": "ok",
  "outputs": [
    {"kind": "text", "label": "UTF-8", "value": "flag{test}"}
  ],
  "warnings": [],
  "score": 0.98
}
```

## 6. 功能覆盖矩阵

以下矩阵按公开功能清单重组为工程模块。实现时以 `registry/operations.toml` 为唯一事实来源，UI 和 CLI 都从注册表读取。

| 模块 | 覆盖内容 | 优先级 | 推荐实现 |
| --- | --- | --- | --- |
| Base 编码 | Base16/32/36/45/58/62/64/85/91/92/100、Base64 隐写、大小写错乱 Base64、混合多重 Base 解码、自定义字典 Base | P0-P1 | Rust |
| ROT | rot5/13/18/47/8000、Rot Special | P0 | Rust |
| 字符与古典密码 | 凯撒、培根、摩斯、栅栏、猪圈、A1Z26、Atbash、Scytale、Caesar Box、Brainfuck、Polybius、键盘密码、敲击码、Baudot、BubbleBabble、Dvorak、NATO、BWT、Hamming、Fernet、Whitespace、Deadfish、Malbolge 等 | P0-P2 | Rust 优先，复杂/冷门项可 Python |
| 中文与趣味编码 | 核心价值观编码、汉字笔画码、阴阳怪气、百家姓、当铺、中文电码、天干地支、六十四卦、佛曰、天书、盲文、零宽字符 | P1-P2 | Rust + 数据表 |
| 编码转换 | Unicode/ASCII、URL、Escape、Bytes、HTML、区位码、ANSI 相关转换 | P0 | Rust |
| 带 key 加解密 | Vigenere、Gronsfeld、Beaufort、Autokey、列移位、列置换、行置换、单表置换、Keyword、Nihilist、OTP、Multiplicative、Porta、Playfair、FracMorse、RC4、滚动密钥 | P1 | Rust |
| 多 key/现代加密 | Hill、ADFGX/ADFGVX、Foursquare、Bifid、Affine、Polybius Square、AES/DES/3DES、Enigma M3、M-209 | P1-P2 | RustCrypto + Rust |
| 进制与二进制 | 2/8/10/16 互转、ASCII 互转、补码反码、Hex/Bytes、BCD、Gray、IEEE754、混合进制、二进制 XOR、ASCII 偏移、长数字串可打印字符分割 | P0-P1 | Rust |
| 其他分析工具 | 字频、英文词频、哈希、HMAC、SHAKE、短 MD4/MD5/CRC32 爆破、时间戳、GCD、拼音、质因数分解、斐波那契解码、SSTI 类序号、pickle 反序列化查看、RSA/PEM/证书解析 | P0-P2 | Rust + Python |
| Crypto 攻击辅助 | RSA 共模攻击、低指数广播攻击、Wiener、费马分解、已知 p/q/e/d 求解、CRT 组合、模逆、离散对数小范围求解、椭圆曲线参数查看、JWT none/弱密钥检测 | P1-P2 | Rust + Python/Sage 适配 |
| 文件与图片 | 文件读写为 bin/oct/hex、Base64 文件互转、图片 Base64、二进制图、RGB 数据图、坐标图、二维码/条形码、GIF 分帧、图片拼接、文件头修复、图片翻转、Hex Viewer | P1 | Rust + Python |
| 隐写与压缩包 | Zip 伪加密/修复/嵌套解压/字典爆破、binwalk/foremost、StegSolve LSB、双图 XOR/OR/AND、盲水印、Stereogram、DTMF、wav 摩斯、PNG LSB、snow、jsteg、steghide、F5、zsteg/outguess、Piet、QRazyBox、Arnold 变换 | P2-P3 | Python 适配器 + 外部工具 |
| 流量与取证 | PCAP 摘要、HTTP/DNS/TCP 流提取、文件还原、TLS SNI/证书提取、USB HID 键盘流量解析、ICMP/DNS 隐写提取、日志时间线 | P2-P3 | Python/scapy + tshark 适配 |
| 逆向与 Pwn 辅助 | ELF/PE/Mach-O 信息查看、strings、熵图、反汇编预览、shellcode 汇编/反汇编、cyclic pattern、ROP gadget 查询、格式化字符串偏移计算、字节序/结构体 pack/unpack | P2-P3 | Rust + Python/pwntools 适配 |
| 密文编辑 | 智能分段、删除/替换空白、删除指定字符、去重、大小写转换、0/1 互换、反转、hex 端序反转、按长度分段、查找替换 | P0 | Rust |
| Web 辅助 | GET 查看、robots 查看、本地 XXE 查看、端口扫描、目录扫描、GET SQLi 轻量检测、HTTP request 数据包解析、请求包转 Python `requests/httpx` 代码、请求包转 curl/JS fetch/Node/Go/Rust 代码、HAR 导入导出、Header/Cookie/Body 编辑、URL 参数与表单参数提取 | P1-P3 | Rust 解析 + 模板生成 |
| Web Token 与 Session | JWT 解析/签名/校验、JWK/JWKS 解析、Flask session 解码与弱密钥测试、Django signed cookie 查看、Rails signed/encrypted cookie 结构识别、PHP session 解析、OAuth/OIDC token 查看 | P1-P3 | Rust + Python |
| 字典与 Payload 辅助 | 字典变形、掩码生成、规则组合、URL 参数 fuzz 列表、弱口令组合、路径字典去重、大小写/分隔符/年份变体、CTF 常见 payload 模板库 | P2 | Rust |
| 题目工作台 | 比赛/题目管理、flag 记录、附件哈希、解题笔记、命令历史、证据截图、临时文件隔离、可导出 writeup 草稿 | P2 | GUI + 本地数据库 |
| 右键菜单 | 复制剪切粘贴、在线搜索、在线翻译、Base64 解码、进制转 ASCII、导出 Hex、保存图文 | P2 | GUI 层 |
| 程序能力 | 一键解码、菜单搜索、结果搜索、Python/JavaScript/Java/PHP 运行器、批处理、历史记录 | P0-P3 | Rust + runner |

## 7. 一键解码设计

一键解码不能简单暴力套所有算法，需要可解释的评分系统。

候选生成：

- 文本特征：字符集、长度、熵、可打印比例、是否符合 Base/Hex/URL/Unicode 形态。
- 结构特征：是否有分隔符、是否全数字、是否成组、是否含零宽字符。
- 文件特征：magic bytes、MIME、EXIF、压缩包目录、PNG chunk、GIF 帧。

评分维度：

- 输出可打印比例。
- UTF-8 合法性。
- flag 正则命中率，可配置如 `flag{}`、`ctf{}`。
- 英文/中文词频得分。
- 熵变化是否合理。
- 多步链路长度惩罚，避免输出大量噪声。

输出形式：

- 展示 Top N 解码路径。
- 每条路径显示步骤、置信度、耗时、警告。
- 支持把某条路径固定为批处理流水线。

## 8. HTTP 请求包解析与代码生成

该模块用于把 Burp Suite、浏览器开发者工具、CTF 题目附件或抓包工具中复制出来的 HTTP request 原始数据包转换为可运行代码，优先支持 Python。

### 8.1 输入格式

支持以下输入：

- 原始 HTTP 请求包，例如 `GET /path?a=1 HTTP/1.1`、Header、空行、Body。
- 只包含 Header 和 Body 的片段，允许用户手动补全 scheme、host、端口。
- curl 命令导入，解析为统一请求模型。
- HAR 文件导入，提取单个或批量请求。
- Burp Suite 复制出的请求，兼容 `Content-Length`、重复 Header、Cookie、multipart form-data。

统一请求模型：

```rust
pub struct HttpRequestModel {
    pub method: String,
    pub scheme: Option<String>,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub cookies: Vec<(String, String)>,
    pub body: RequestBody,
}
```

### 8.2 输出代码

第一优先级输出：

- Python `requests` 同步代码。
- Python `httpx` 同步/异步代码。
- Python 带代理、超时、重定向、verify、CookieJar、multipart 上传的模板。

后续输出：

- curl 命令。
- JavaScript `fetch`。
- Node.js `axios`。
- Go `net/http`。
- Rust `reqwest`。
- YAML/JSON 请求描述，便于保存为批处理任务。

Python `requests` 输出示例：

```python
import requests

url = "https://example.com/login"
headers = {
    "User-Agent": "Mozilla/5.0",
    "Content-Type": "application/x-www-form-urlencoded",
}
cookies = {
    "session": "REPLACE_ME",
}
data = {
    "username": "admin",
    "password": "admin",
}

response = requests.post(
    url,
    headers=headers,
    cookies=cookies,
    data=data,
    timeout=10,
    allow_redirects=False,
)
print(response.status_code)
print(response.text)
```

### 8.3 编辑与增强能力

- 自动拆分 URL query、form body、JSON body、multipart body。
- Cookie 字符串转 dict，支持按域名保存 Cookie 模板。
- Header 大小写保留，自动提示可删除的无效 Header，如过期 `Content-Length`。
- 支持把 `application/json`、`x-www-form-urlencoded`、multipart、raw bytes 分别生成合适代码。
- 支持批量变量化参数，例如把 `id=1` 标记为 payload 位置。
- 支持生成带代理的代码，方便配合 Burp：`proxies={"http": "...", "https": "..."}`。
- 支持生成重放脚本、弱口令循环模板、参数 fuzz 模板，但默认不内置破坏性 payload。
- 支持响应包解析，把 raw response 转为状态码、Header、Cookie、Body 和保存文件代码。

### 8.4 安全策略

- 默认只生成代码，不自动发包。
- 点击重放前必须确认目标 URL。
- 默认脱敏 `Cookie`、`Authorization`、`X-Api-Key`、`Token`、`Set-Cookie` 等敏感字段。
- 生成代码时给敏感值写成 `REPLACE_ME`，用户可选择保留原值。
- 历史记录默认不保存完整认证信息。
- 代码生成模板不包含未授权扫描、破坏性 SQLi payload 或凭据窃取逻辑。

### 8.5 Operation 规划

| Operation ID | 功能 | 优先级 |
| --- | --- | --- |
| `http.raw.parse` | 原始请求包解析为统一模型 | P1 |
| `http.raw.to_python_requests` | 请求包转 Python `requests` 代码 | P1 |
| `http.raw.to_python_httpx` | 请求包转 Python `httpx` 代码 | P1 |
| `http.raw.to_curl` | 请求包转 curl | P1 |
| `http.raw.to_fetch` | 请求包转 JavaScript fetch | P2 |
| `http.raw.to_go` | 请求包转 Go `net/http` | P2 |
| `http.raw.to_rust_reqwest` | 请求包转 Rust `reqwest` | P2 |
| `http.har.import` | HAR 导入并选择请求 | P2 |
| `http.response.parse` | 原始响应包解析 | P2 |
| `http.request.replay` | 手动确认后重放请求 | P3 |

## 9. 实战增强功能池

这些能力不一定都在第一阶段实现，但应该提前纳入架构，避免后期重构。

| 功能组 | 具体能力 | 建议优先级 | 实现建议 |
| --- | --- | --- | --- |
| JWT 与 Web Session | JWT decode/verify/sign、none 算法检测、弱密钥字典测试、JWK/JWKS 解析、Flask/Django/Rails/PHP session 查看 | P1-P2 | Rust 解析，Python 适配特殊框架 |
| RSA/数论工具 | 大整数计算、模逆、CRT、共模攻击、低指数广播攻击、Wiener、费马分解、已知参数补全、PEM/JWK 转换 | P1-P2 | Rust 大整数 + Python/Sage 可选 |
| PCAP 流量分析 | HTTP 对象提取、DNS/ICMP 隐写、TCP stream 重组、USB HID 键盘解析、TLS SNI/证书提取、文件还原 | P2-P3 | scapy/tshark 适配 |
| 逆向辅助 | ELF/PE/Mach-O 头解析、strings、熵图、导入导出表、节区查看、反汇编预览、YARA/正则匹配 | P2-P3 | Rust parser + capstone 适配 |
| Pwn 辅助 | cyclic pattern、偏移定位、ROP gadget 查询、one_gadget 适配、shellcode asm/disasm、格式化字符串计算、pack/unpack | P2-P3 | Rust + pwntools/ROPgadget 适配 |
| 字典生成 | 掩码、规则、大小写变体、年份/生日/分隔符组合、路径字典清洗去重、hashcat/john 格式导出 | P2 | Rust 并行生成 |
| Payload 模板库 | SQLi/XSS/SSTI/XXE/反序列化/模板注入常见 CTF payload 分类检索，默认仅生成示例不自动攻击 | P2 | 本地 YAML 模板 |
| 附件自动分诊 | magic bytes、熵、binwalk 摘要、strings 摘要、EXIF、压缩包嵌套、可疑编码片段提取 | P1-P2 | Rust 主导，Python 补充 |
| 题目工作台 | 比赛空间、题目状态、flag、附件、笔记、命令历史、截图、临时目录、writeup 导出 | P2 | SQLite + GUI |
| 插件市场 | 本地插件安装、版本约束、能力声明、权限声明、测试样例、禁用与卸载 | P3 | Manifest + 沙箱策略 |

优先级建议：

- 先做 request 转 Python、JWT、RSA/数论、附件自动分诊。
- 再做 PCAP、Pwn/逆向基础、字典生成。
- 最后做插件市场、复杂 payload 库和跨题目知识库。

## 10. 性能要求

基础目标：

- 10 MB 文本 Base64/Hex/URL 转换在普通笔记本上应接近实时。
- 1 GB 文件 hash 使用流式读取，不整体载入内存。
- 批量文件分析默认使用 CPU 核心数并发，但 UI 不阻塞。
- 所有长任务支持取消、进度、超时。

实现原则：

- 高频算法在 Rust 中实现，不走 Python。
- 文件处理使用 reader/writer 流式接口。
- 大文件优先使用 `memmap2` 或分块处理。
- 哈希与爆破任务用 `rayon` 并行。
- Python worker 按任务批量接收输入，避免逐字符 RPC。
- 外部工具调用必须限制工作目录、运行时间和输出大小。

基准测试：

- `criterion` 覆盖 Base、Hex、XOR、hash、常见古典密码。
- 每个 release 记录 benchmark 结果。
- 关键性能退化超过 10% 需要解释或回滚。

## 11. 安全边界

本工具面向合法 CTF 和授权测试场景。安全边界必须写进代码：

- Web 扫描模块默认只允许用户显式输入的目标，不做网段自动扩散。
- 端口扫描默认低并发、可取消、有速率限制。
- SQLi 检测只做轻量探测和回显分析，不内置破坏性 payload。
- HTTP 请求包转代码默认脱敏认证信息，默认只生成代码，不自动发送。
- Python/JS/Java/PHP 运行器默认隔离工作目录，禁止继承敏感环境变量。
- 第三方命令运行器默认关闭，首次使用需要用户确认风险。
- Pwn/逆向模块默认只处理本地样本，不自动连接远程服务。
- 历史记录不得保存用户明确标记为敏感的输入。

## 12. UI 设计

主界面采用工具型布局：

- 左侧：分类树、搜索、收藏。
- 中间：输入区、参数区、运行按钮、批处理队列。
- 右侧：结果、候选路径、解释、导出。
- 底部：任务进度、日志、错误和性能信息。

关键交互：

- 全局搜索支持中文名、英文名、别名、常见拼写错误。
- 输入区支持文本、文件拖拽、图片粘贴。
- 结果支持复制、保存、再次作为输入、加入流水线。
- 每个操作显示参数说明、示例和安全级别。
- 一键解码结果必须展示路径，不只展示最终文本。
- Web 请求包工具支持左右分栏：左侧 raw request/response，右侧代码预览、参数表、脱敏开关和生成语言切换。
- 题目工作台支持按比赛、题目、附件、笔记、命令历史组织上下文，避免比赛中散落文件。

## 13. CLI 设计

CLI 与 GUI 共用注册表。

示例：

```bash
ctf-tools list --category base
ctf-tools run base64.decode --text 'ZmxhZ3t0ZXN0fQ=='
ctf-tools run xor.bruteforce-byte --file cipher.bin --limit 200
ctf-tools auto --text '666c61677b746573747d'
ctf-tools file hex-view sample.png --offset 0 --length 256
ctf-tools http to-python-requests --file request.txt --redact-secrets
ctf-tools http to-curl --file request.txt
ctf-tools jwt decode --text "$TOKEN"
ctf-tools crypto rsa-common-modulus --n n.txt --e1 3 --c1 c1.txt --e2 65537 --c2 c2.txt
ctf-tools pcap extract-http --file traffic.pcapng --out artifacts/
ctf-tools pwn cyclic --length 400
ctf-tools batch pipeline.toml samples/
```

流水线示例：

```toml
[[steps]]
op = "base64.decode"

[[steps]]
op = "xor.bruteforce-byte"
params.max_key = 200
```

## 14. 测试策略

每个 Operation 至少要有：

- 正向测试：标准输入得到标准输出。
- 反向测试：encode/decode 可逆时做 round-trip。
- 错误测试：非法输入、空输入、超长输入。
- Golden tests：真实 CTF 样本和固定结果。

重点测试工具：

- `cargo test`：Rust 单元与集成测试。
- `proptest`：编码转换、XOR、分段等适合性质测试的模块。
- `cargo fuzz`：Base/URL/Unicode/文件解析等输入面大的模块。
- `pytest`：Python 插件和外部工具适配器。
- UI snapshot：关键面板渲染和操作搜索结果。
- HTTP request 解析测试：重复 Header、Cookie、multipart、JSON、chunked、非 UTF-8 body、缺失 Host、Burp/HAR/curl 互转。
- Crypto 攻击辅助测试：使用公开小参数测试向量，避免把重型爆破放入默认测试。

## 15. 发布与打包

目标平台：

- macOS arm64/x64。
- Windows x64。
- Linux x64。

发布形态：

- GUI 安装包。
- 单独 CLI 二进制。
- Python 插件包和第三方工具适配器包。

版本策略：

- `0.x` 阶段按模块交付。
- 每个版本记录新增 Operation 数量、兼容变更、性能变化和已知问题。
- 注册表增加 `since`、`stability`、`backend` 字段，便于追踪覆盖率。

## 16. 里程碑

### M0：工程骨架

- 建立 Cargo workspace、Python worker、注册表格式。
- 完成 GUI 空壳、CLI 空壳、Operation 执行链路。
- 建立测试、lint、benchmark 基线。

### M1：文本编码基础

- Base、ROT、URL、HTML、Unicode、Hex/Bytes、进制转换。
- 密文编辑模块。
- 操作搜索、历史记录、结果复用。

### M2：密码学基础

- 常见古典密码、带 key 加解密、哈希/HMAC/SHAKE。
- XOR 爆破、短 CRC32/MD5/MD4 爆破。
- RSA/数论基础、JWT 解析与校验。
- 初版一键解码评分器。

### M3：文件与图片

- Hex Viewer、文件 Base64、图片 Base64、GIF 分帧、图片拼接。
- 二维码/条形码识别。
- Zip 修复、嵌套解压、文件头修复。

### M4：隐写与音频

- LSB、双图 XOR/OR/AND、盲水印适配器。
- DTMF、wav 摩斯、snow、jsteg、steghide、zsteg/outguess 适配器。
- 样本库与批处理。

### M5：Web 与运行器

- HTTP request 数据包解析、请求包转 Python `requests/httpx`、curl、fetch。
- JWT、Flask/Django/Rails/PHP session 查看与弱密钥测试。
- GET/robots/目录扫描/端口扫描/轻量 SQLi 检测。
- Python/JavaScript/Java/PHP 运行器。
- 第三方命令运行器安全策略。

### M6：实战增强

- PCAP 流量分析、HTTP/DNS/USB HID 提取。
- 逆向/Pwn 辅助、字典生成、附件自动分诊。
- 题目工作台、writeup 草稿导出。

### M7：全量覆盖与发布

- 对照公开功能清单补齐缺口。
- 完成跨平台打包。
- 完成用户文档、开发者插件文档和示例题库。

## 17. 风险与对策

| 风险 | 影响 | 对策 |
| --- | --- | --- |
| 功能范围过大 | 交付周期不可控 | 注册表驱动，按 P0-P3 分批交付 |
| 冷门算法资料不一致 | 输出和用户预期不一致 | 每个算法附测试向量和参考规范链接 |
| Python 依赖难打包 | 发布包复杂 | Python worker 独立，核心功能不依赖 Python |
| 第三方工具不可用 | 隐写模块体验不稳定 | 适配器检测、降级提示、可配置路径 |
| 一键解码误报多 | 用户无法判断结果 | 输出置信度、路径、证据和排序原因 |
| Web 模块被误用 | 法律和安全风险 | 速率限制、授权提示、默认保守 payload |
| 请求包转代码泄露认证信息 | Cookie/Token 被历史记录或代码片段保存 | 默认脱敏、敏感字段检测、保存前提示 |
| Pwn/逆向依赖复杂 | 跨平台安装失败 | 核心能力 Rust 内置，pwntools/capstone/ROPgadget 走可选适配器 |

## 18. 开发规范

- 每个新功能必须先登记到 `registry/operations.toml`。
- Rust Operation 必须有单元测试和至少一个 CLI 集成测试。
- Python 插件必须有超时测试和异常输出测试。
- 不允许 UI 直接调用具体算法实现，只能通过 `OperationRunner`。
- 不允许核心算法依赖网络。
- 不允许把临时文件写到用户输入目录，统一写入任务工作目录。
- 错误信息要可行动，例如提示输入格式、参数范围、依赖缺失。
- 请求包代码生成必须保留用户可审计的中间模型，不能直接对 raw 文本做字符串拼接输出。
- 涉及凭据、Cookie、Token 的 Operation 必须声明敏感字段策略和历史记录策略。

## 19. 调研来源

本开发文档使用以下公开网页作为功能范围调研来源，后续仍需要把功能清单固化为机器可读注册表：

- HackTwoHub：《随波逐流 CTF 编码工具 v7.2，一款搞定所有 CTF 编码 / 隐写难题》  
  https://www.hacktwohub.com/1626.html
- ZONE.CI：《2026开年王炸：随波逐流CTF工具7.2，把CTF打成Easy》  
  https://security.zone.ci/secarticles/wx/484837.html
- 随波逐流信息安全网：`bo_ctfcode.html` 公开页面在本次访问时返回 502，但搜索索引摘要显示其为 v7.2/v7.21 发布页之一。  
  https://1o1o.xyz/bo_ctfcode.html
- 本机 TscanPlus v3.2.5 界面：仅读取可见功能分区和产品形态，未执行扫描、发包、删除、配置修改或高风险命令。
