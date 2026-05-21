# CyberChef 编码能力对照

对照来源：GCHQ CyberChef 官方仓库 `src/core/config/Categories.json` 的 `Data format` 分类与 `src/core/operations` 中对应操作。

## 已覆盖

- To/From Base64
- To/From Base32
- To/From Base45
- To/From Base58
- To/From Base62
- To/From Base85
- To/From Hex
- To/From Binary
- To/From Octal
- To/From Decimal
- To/From Charcode：当前对应 ASCII 编码/解码
- To/From HTML Entity
- URL Encode / URL Decode
- Escape / Unescape Unicode Characters
- To/From Quoted Printable

## 仍需补齐

- To/From Hexdump：当前只有 Hex Viewer，还缺 CyberChef 风格 hexdump 文本反解。
- Show Base64 offsets：适合并入 Base64 工具的分析视图。
- To/From Base92：CTF 中偶尔出现，优先级 P2。
- To/From Bech32：区块链地址相关，优先级 P2。
- To/From Base：通用任意 alphabet/base 转换，优先级 P1。
- To/From BCD：嵌入式和流量题可能出现，优先级 P2。
- Normalise Unicode：用于同形字符、组合字符和宽窄字符规范化，优先级 P1。
- To/From Punycode：域名题常见，优先级 P1。
- Encode text / Decode text / Text Encoding Brute Force：字符集转换与编码爆破，优先级 P1。
- Swap endianness：更偏二进制辅助，优先级 P1。
- To/From Braille：趣味编码，优先级 P2。
- To/From Modhex：YubiKey/键盘映射相关，优先级 P2。
- MIME Decoding：邮件/HTTP 头相关，优先级 P2。

## 暂不归入“编码”核心的 Data format 项

- AMF、MessagePack、CBOR、CSV/JSON/YAML、Avro、Rison、TLV、ASN.1、PEM/Hex。
- 这些更像结构化格式解析/转换，后续应放入文件/结构化数据模块，而不是和文本编码混在一起。
