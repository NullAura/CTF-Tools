# MVP 实施计划

日期：2026-05-21  
目标：用最短路径做出可运行、可扩展、可测试的 CTF 工具箱骨架。

## 1. MVP 定义

MVP 不追求一次覆盖所有工具，而是先打通三条主链路：

- `OperationRegistry`：功能注册、搜索、参数描述、执行分发。
- `Rust Core + CLI`：核心算法、文件处理、HTTP request 解析和代码生成。
- `Python Worker`：为后续隐写、PCAP、Pwn/逆向适配器留好边界。

第一版必须能完成：

- 文本编码解码：Base64、Base32、URL、HTML、Unicode、ASCII、Hex、进制转换。
- 哈希：MD5、SHA1、SHA2、SHA3、NTLM、SM3。
- 加解密：XOR、AES、DES、3DES、RSA 基础参数解析。
- HTTP request 数据包转 Python `requests/httpx`、curl。
- JWT 解析、签名校验、none 算法检测、弱密钥字典测试。
- 资产文本分拣：URL、主域名、子域名、IP、C 段、邮箱、手机号、身份证格式、Tscan/Fscan 结果摘要。
- CLI、GUI 共用同一套 Operation。

## 2. 第一阶段目录

```text
ctf-tools/
  Cargo.toml
  crates/
    ctf-core/
    ctf-codecs/
    ctf-crypto/
    ctf-web/
    ctf-runner/
    ctf-cli/
    ctf-app/
  python/
    pyproject.toml
    ctf_toolbox_worker/
  registry/
    operations.toml
  docs/
```

## 3. 开发顺序

### 第 1 步：工程骨架

- 创建 Cargo workspace。
- 建立 `ctf-core` 的 `OperationSpec`、`OperationInput`、`OperationOutput`、`OperationError`。
- 读取 `registry/operations.toml`，校验字段完整性。
- CLI 支持 `list`、`search`、`run` 三个命令。

验收：

- `ctf-tools list` 能列出注册表功能。
- `ctf-tools search base64` 能命中中文名、英文名、别名。
- `cargo test` 通过。

### 第 2 步：编码与哈希

- 实现 Base64/Base32/URL/HTML/Unicode/ASCII/Hex。
- 实现 MD5/SHA1/SHA2/SHA3/NTLM/SM3。
- 实现输入输出类型：text、bytes、file。
- 所有 encode/decode 支持批量模式。

验收：

- 每个 Operation 有测试向量。
- 10 MB Base64 decode 不阻塞 CLI。
- 错误输入返回结构化错误，不 panic。

### 第 3 步：HTTP request 转代码

- 原始 HTTP 请求包解析为 `HttpRequestModel`。
- 解析 query、headers、cookies、body。
- 生成 Python `requests`、Python `httpx`、curl。
- 默认脱敏 Cookie、Authorization、Token。

验收：

- 支持 GET/POST/JSON/form/multipart/raw body。
- 支持 Burp 复制出来的 raw request。
- 支持缺失 scheme 时手动传入 `--scheme https`。
- 生成的 Python 代码能被 `python -m py_compile` 校验。

### 第 4 步：JWT 与资产分拣

- JWT decode、header/payload 美化、签名校验。
- HS256 弱密钥字典测试，默认小字典，用户可指定字典文件。
- 资产分拣提取 URL、域名、IP、C 段、邮箱、手机号。
- 支持 Tscan/Fscan 结果摘要提取。

验收：

- 不保存 JWT 原文到历史记录，除非用户显式开启。
- 弱密钥测试可取消、可超时。
- 资产分拣输出可复制为分组文本或 JSON。

### 第 5 步：GUI

- 左侧分类树和搜索。
- 中间输入与参数区。
- 右侧结果区。
- 支持拖拽文件、复制结果、结果再次作为输入。

验收：

- GUI 能调用同一个 Operation runner。
- 长任务有进度、取消按钮和错误提示。
- HTTP request 工具有 raw request 与代码预览分栏。

## 4. 首批不可做事项

MVP 阶段不做：

- 主动漏洞扫描。
- 自动连接远程服务的 Pwn 模块。
- 真实攻击 payload 自动执行。
- 复杂隐写外部工具打包。
- 在线空间测绘 API 查询。

这些能力只保留注册表、权限模型和接口设计，等核心稳定后再实现。

## 5. 验收标准

- 所有 P0/P1 Operation 至少有一条标准测试向量。
- CLI 和 GUI 行为一致。
- 所有涉及 Cookie、Token、Authorization 的功能默认脱敏。
- 所有网络请求默认需要用户显式输入目标，且有超时。
- Python worker 崩溃不影响主程序。
- 注册表、文档、测试三者能互相对应。

