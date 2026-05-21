# TscanPlus 对标记录

日期：2026-05-21  
来源：通过本机已运行的 TscanPlus v3.2.5 界面读取功能分区，仅用于产品功能对标。

## 1. 可见功能分区

TscanPlus 当前可见顶层分区：

- 项目管理
- 信息收集
- 资产探测
- 漏洞检测
- 轻武器库
- 空间测绘
- 编码转换
- 红队常用
- 辅助工具
- 快捷启动
- About

这些分区说明 CTF 工具箱不应只做编码解码，还应考虑资产文本处理、请求重放、空间测绘语法、项目工作台和本地工具启动。

## 2. 可借鉴的安全功能形态

可以纳入本项目的能力：

- 项目维度管理目标、附件、结果和配置。
- 信息收集结果的结构化展示：备案、域名、端口、子域名、历史 IP、APP、小程序。
- 资产探测入口：端口扫描、Web 指纹、域名枚举、目录枚举、JsFinder、Swagger 分析。
- Repeater：raw HTTP request/response 编辑、美化、编码选择、代理开关、响应体长度限制。
- 编码转换：Base64/Base32/URL/文本编码/ASCII/进制/HTML/Unicode、哈希、AES/RSA/SM2/SM4/DES/3DES/XOR。
- 辅助工具：资产分拣、数据处理、密码生成、密码查询、杀软查询、提权辅助。
- 空间测绘语法转换：FOFA、Hunter、Quake、ZoomEye、Shodan、Censys、0.zone、DaydayMap、VT。
- 轻武器库中的 JWT、40x Bypass、Host 碰撞、ICP/IP 查询、代理池、小程序反编译等功能入口。

## 3. 本项目应增强的地方

相比对标工具，本项目应重点增强：

- CTF 编码和隐写覆盖度，保留“随波逐流工具箱”式的一键解码和大量冷门编码。
- HTTP request 转 Python `requests/httpx`、curl、fetch、Go、Rust 代码。
- 所有功能走 `OperationRegistry`，CLI 与 GUI 共享。
- 插件权限模型，区分本地分析、联网查询、外部命令、高风险模板。
- 默认脱敏 Cookie、Token、Authorization。
- 批处理流水线和可解释的一键解码路径。
- 题目工作台，支持 writeup 草稿导出。

## 4. 不直接复刻的内容

以下内容不进入默认发行包：

- 凭据获取、权限维持、远控上线、绕过安全产品等高风险命令模板。
- 会自动对第三方目标发起未授权扫描的默认流程。
- 依赖用户账号 API 的空间测绘自动查询，除非用户显式配置并确认授权。
- 任何闭源工具的代码、资源、规则库和私有实现。

## 5. 建议映射到本项目的模块

| TscanPlus 功能形态 | 本项目模块 | 处理方式 |
| --- | --- | --- |
| 编码转换 | `ctf-codecs`、`ctf-crypto` | 主线 P0/P1 |
| Repeater | `ctf-web` | 主线 P1，增加 request 转代码 |
| 资产分拣 | `ctf-web` 或 `ctf-core` 文本提取 | 主线 P1 |
| 空间测绘语法 | `template_pack` 插件 | P2，默认只做语法转换 |
| Swagger 分析 | `ctf-web` | P2，默认不自动探测 |
| JWT | `ctf-crypto`、`ctf-web` | 主线 P1 |
| 小程序反编译 | Python/外部工具插件 | P3，本地文件分析 |
| 红队命令库 | 用户自建模板库 | 默认不内置高风险模板 |
| 快捷启动 | GUI 工具启动器 | P3 |

