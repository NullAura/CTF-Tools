# 插件与适配器模型

日期：2026-05-21

## 1. 设计目标

插件系统用于承载非核心、依赖复杂或更新频繁的能力：

- Python 隐写、PCAP、图像、音频、Pwn、逆向适配器。
- 外部工具封装，如 `binwalk`、`foremost`、`zsteg`、`steghide`、`tshark`、`capstone`、`pwntools`。
- 本地模板库，如 payload、字典规则、空间测绘查询语法。

核心原则：

- 核心编码、哈希、基础密码算法不依赖插件。
- 插件必须声明权限、输入输出类型、依赖和测试样例。
- 插件默认不能联网、不能访问任意文件、不能长期驻留后台。

## 2. 插件类型

| 类型 | 用途 | 示例 |
| --- | --- | --- |
| `python_worker` | Python 算法或生态库 | PCAP、图片隐写、音频 DTMF |
| `external_command` | 外部二进制封装 | tshark、binwalk、zsteg |
| `template_pack` | 本地模板库 | payload、红队命令笔记、空间测绘语法 |
| `ui_extension` | GUI 扩展面板 | 题目工作台、插件管理页 |

## 3. Manifest

插件必须提供 `plugin.toml`：

```toml
id = "pcap.scapy"
name = "PCAP Scapy Adapter"
version = "0.1.0"
kind = "python_worker"
description = "Extract HTTP/DNS/ICMP artifacts from PCAP files."

[permissions]
network = false
read_files = true
write_files = true
execute_process = false
secrets = false

[[operations]]
id = "pcap.http.extract"
name_zh = "PCAP HTTP 文件提取"
name_en = "Extract HTTP Objects"
input = ["file"]
output = ["artifact_dir", "json"]
safety = "local_only"
```

## 4. 权限模型

| 权限 | 默认 | 说明 |
| --- | --- | --- |
| `network` | false | 是否允许主动联网 |
| `read_files` | false | 是否允许读取用户指定文件 |
| `write_files` | false | 是否允许写出结果 |
| `execute_process` | false | 是否允许执行第三方命令 |
| `secrets` | false | 是否允许读取或保留 Cookie/Token |
| `dangerous_templates` | false | 是否允许展示高风险命令模板 |

权限提升必须由用户在插件管理页显式开启。即使插件声明了权限，也不代表默认启用。

## 5. Python Worker 协议

Rust 主进程通过 JSON Lines 调用 Python worker。

请求：

```json
{
  "id": "task-1",
  "operation": "pcap.http.extract",
  "input": {"kind": "file", "path": "/tmp/a.pcapng"},
  "params": {"max_objects": 100},
  "limits": {"timeout_ms": 30000, "max_output_bytes": 104857600}
}
```

响应：

```json
{
  "id": "task-1",
  "status": "ok",
  "outputs": [
    {"kind": "json", "value": {"objects": 3}},
    {"kind": "artifact_dir", "path": "/tmp/ctf-tools/task-1"}
  ],
  "warnings": []
}
```

## 6. 外部命令适配器

外部命令必须通过 `ctf-runner` 启动：

- 固定工作目录。
- 显式参数数组，不拼接 shell 字符串。
- 超时、最大 stdout/stderr、最大输出文件大小。
- 环境变量白名单。
- 结果文件必须写入任务目录。

禁止插件直接执行用户输入拼接出来的命令。

## 7. 高风险模板处理

工具可以提供“命令笔记/模板库”的产品能力，但默认策略是：

- 默认只内置防御、诊断、CTF 本地实验和合法授权场景模板。
- 凭据获取、权限维持、绕过检测、远控上线等高风险模板不进入默认发行包。
- 用户自建模板可以本地保存，但必须标记来源和风险等级。
- 模板不会自动执行，只能复制或生成本地笔记。

## 8. 插件测试

每个插件必须提供：

- `manifest` 校验测试。
- 至少一个 fixture 输入。
- 超时测试。
- 错误输入测试。
- 权限缺失时的失败测试。

