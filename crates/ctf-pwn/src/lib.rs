//! Pwn, reverse engineering, and challenge workspace helpers.

use ctf_core::{
    CtfError, OperationOutput, OperationRequest, OperationResponse, OperationRunner, OperationSpec,
    Result,
};

const DEFAULT_CYCLIC_ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

pub fn register_handlers(runner: &mut OperationRunner) {
    runner.register_handler("binary.info", binary_info);
    runner.register_handler("reverse.strings", reverse_strings);
    runner.register_handler("pwn.cyclic.create", cyclic_create);
    runner.register_handler("pwn.cyclic.offset", cyclic_offset);
    runner.register_handler("pwn.pack.u64_le", pack_u64_le);
    runner.register_handler("pwn.unpack.u64_le", unpack_u64_le);
    runner.register_handler("shellcode.asm.x86_64", shellcode_asm_x86_64);
    runner.register_handler("shellcode.disasm.x86_64", shellcode_disasm_x86_64);
    runner.register_handler("workspace.writeup.template", writeup_template);
}

fn binary_info(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let bytes = request.input_bytes()?;
    let value = if bytes.starts_with(b"\x7fELF") {
        elf_info(&bytes)
    } else if bytes.starts_with(b"MZ") {
        pe_info(&bytes)
    } else if let Some(info) = macho_info(&bytes) {
        info
    } else {
        serde_json::json!({
            "format": "unknown",
            "size": bytes.len(),
            "magic": hex_prefix(&bytes, 16),
        })
    };

    json_output("binary-info", value)
}

fn reverse_strings(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let bytes = request.input_bytes()?;
    let ascii = extract_ascii_strings(&bytes, 4);
    let utf16le = extract_utf16le_strings(&bytes, 4);
    json_output(
        "strings",
        serde_json::json!({
            "ascii": ascii,
            "utf16le": utf16le,
        }),
    )
}

fn cyclic_create(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let length = parse_requested_length(&request.input_text()?, request.limits.max_output_bytes)?;
    let pattern = cyclic_pattern(length);
    Ok(single_output(
        "text",
        "cyclic",
        String::from_utf8(pattern).map_err(|_| CtfError::InvalidUtf8)?,
    ))
}

fn cyclic_offset(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let text = request.input_text()?;
    let pattern = cyclic_pattern(32768);
    let candidates = needle_candidates(&text);
    let mut matches = Vec::new();

    for (kind, needle) in candidates {
        if needle.is_empty() {
            continue;
        }
        if let Some(offset) = find_subsequence(&pattern, &needle) {
            matches.push(serde_json::json!({
                "kind": kind,
                "needle_hex": hex::encode(&needle),
                "offset": offset,
            }));
        }
    }

    json_output(
        "cyclic-offset",
        serde_json::json!({
            "matches": matches,
            "searched": pattern.len(),
        }),
    )
}

fn pack_u64_le(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let values = parse_numbers(&request.input_text()?)?;
    let mut bytes = Vec::with_capacity(values.len() * 8);
    for value in &values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    json_output(
        "pack-u64-le",
        serde_json::json!({
            "values": values,
            "hex": hex::encode(&bytes),
            "escaped": escaped_bytes(&bytes),
        }),
    )
}

fn unpack_u64_le(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let mut warnings = Vec::new();
    let bytes = parse_byte_text(&request.input_text()?);
    if bytes.len() < 8 {
        return Err(CtfError::InvalidInput(
            "need at least 8 bytes or 16 hex characters".to_string(),
        ));
    }
    if !bytes.len().is_multiple_of(8) {
        warnings.push(format!(
            "input length {} is not a multiple of 8; trailing bytes ignored",
            bytes.len()
        ));
    }
    let values = bytes
        .chunks_exact(8)
        .map(|chunk| {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(chunk);
            let value = u64::from_le_bytes(buf);
            serde_json::json!({
                "decimal": value,
                "hex": format!("0x{value:016x}"),
            })
        })
        .collect::<Vec<_>>();

    let mut response = json_response("unpack-u64-le", serde_json::json!({ "values": values }))?;
    response.warnings = warnings;
    Ok(response)
}

fn shellcode_asm_x86_64(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let mut bytes = Vec::new();
    for line in request.input_text()?.lines() {
        let line = strip_comment(line).trim();
        if line.is_empty() {
            continue;
        }
        bytes.extend(assemble_x86_64_line(line)?);
    }
    json_output(
        "asm-x86-64",
        serde_json::json!({
            "hex": hex::encode(&bytes),
            "escaped": escaped_bytes(&bytes),
            "bytes": bytes.len(),
        }),
    )
}

fn shellcode_disasm_x86_64(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let bytes = parse_byte_text(&request.input_text()?);
    Ok(single_output(
        "text",
        "disasm-x86-64",
        disassemble_x86_64(&bytes),
    ))
}

fn writeup_template(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let text = request.input_text()?;
    let title = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(str::trim)
        .unwrap_or("challenge");
    Ok(single_output(
        "text",
        "writeup-template",
        format!(
            "# {title}\n\n## Summary\n\n## Environment\n\n## Recon\n\n## Exploit Path\n\n## Flag\n\n## Notes\n\n## Commands\n\n```sh\n# record useful commands here\n```\n"
        ),
    ))
}

fn elf_info(bytes: &[u8]) -> serde_json::Value {
    let class = match bytes.get(4) {
        Some(1) => "ELF32",
        Some(2) => "ELF64",
        _ => "unknown",
    };
    let endian = match bytes.get(5) {
        Some(1) => "little",
        Some(2) => "big",
        _ => "unknown",
    };
    let machine = read_u16(bytes, 18, endian)
        .map(elf_machine)
        .unwrap_or("unknown");
    serde_json::json!({
        "format": "ELF",
        "class": class,
        "endian": endian,
        "machine": machine,
        "size": bytes.len(),
    })
}

fn pe_info(bytes: &[u8]) -> serde_json::Value {
    let pe_offset = read_u32_le(bytes, 0x3c).map(|value| value as usize);
    let (machine, sections) = pe_offset
        .filter(|offset| bytes.get(*offset..offset + 4) == Some(b"PE\0\0".as_slice()))
        .map(|offset| {
            (
                read_u16_le(bytes, offset + 4)
                    .map(pe_machine)
                    .unwrap_or("unknown"),
                read_u16_le(bytes, offset + 6).unwrap_or(0),
            )
        })
        .unwrap_or(("unknown", 0));
    serde_json::json!({
        "format": "PE",
        "machine": machine,
        "sections": sections,
        "size": bytes.len(),
    })
}

fn macho_info(bytes: &[u8]) -> Option<serde_json::Value> {
    let magic = read_u32_be(bytes, 0)?;
    let (format, endian, bits) = match magic {
        0xfeedface => ("Mach-O", "big", 32),
        0xcefaedfe => ("Mach-O", "little", 32),
        0xfeedfacf => ("Mach-O", "big", 64),
        0xcffaedfe => ("Mach-O", "little", 64),
        0xcafebabe => ("Mach-O Fat", "big", 0),
        0xbebafeca => ("Mach-O Fat", "little", 0),
        _ => return None,
    };
    Some(serde_json::json!({
        "format": format,
        "endian": endian,
        "bits": bits,
        "size": bytes.len(),
    }))
}

fn extract_ascii_strings(bytes: &[u8], min_len: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = Vec::new();
    for byte in bytes {
        if byte.is_ascii_graphic() || *byte == b' ' {
            current.push(*byte);
        } else {
            push_ascii_string(&mut result, &mut current, min_len);
        }
    }
    push_ascii_string(&mut result, &mut current, min_len);
    result
}

fn extract_utf16le_strings(bytes: &[u8], min_len: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    for chunk in bytes.chunks_exact(2) {
        if chunk[1] == 0 && (chunk[0].is_ascii_graphic() || chunk[0] == b' ') {
            current.push(chunk[0] as char);
        } else if current.len() >= min_len {
            result.push(std::mem::take(&mut current));
        } else {
            current.clear();
        }
    }
    if current.len() >= min_len {
        result.push(current);
    }
    result
}

fn push_ascii_string(result: &mut Vec<String>, current: &mut Vec<u8>, min_len: usize) {
    if current.len() >= min_len {
        result.push(String::from_utf8_lossy(current).into_owned());
    }
    current.clear();
}

fn parse_requested_length(text: &str, max_output_bytes: usize) -> Result<usize> {
    let length = text
        .split(|char: char| !char.is_ascii_digit())
        .find(|part| !part.is_empty())
        .map(|part| part.parse::<usize>())
        .transpose()
        .map_err(|error| CtfError::InvalidInput(error.to_string()))?
        .unwrap_or(128);
    if length > max_output_bytes {
        return Err(CtfError::OutputLimitExceeded {
            limit: max_output_bytes,
        });
    }
    Ok(length)
}

fn cyclic_pattern(length: usize) -> Vec<u8> {
    let sequence = de_bruijn(DEFAULT_CYCLIC_ALPHABET, 4);
    sequence.into_iter().cycle().take(length).collect()
}

fn de_bruijn(alphabet: &[u8], n: usize) -> Vec<u8> {
    fn db(
        t: usize,
        p: usize,
        k: usize,
        n: usize,
        a: &mut [usize],
        alphabet: &[u8],
        sequence: &mut Vec<u8>,
    ) {
        if t > n {
            if n.is_multiple_of(p) {
                for index in 1..=p {
                    sequence.push(alphabet[a[index]]);
                }
            }
        } else {
            a[t] = a[t - p];
            db(t + 1, p, k, n, a, alphabet, sequence);
            for j in a[t - p] + 1..k {
                a[t] = j;
                db(t + 1, t, k, n, a, alphabet, sequence);
            }
        }
    }

    let k = alphabet.len();
    let mut a = vec![0; k * n + 1];
    let mut sequence = Vec::new();
    db(1, 1, k, n, &mut a, alphabet, &mut sequence);
    sequence
}

fn needle_candidates(text: &str) -> Vec<(String, Vec<u8>)> {
    let trimmed = text.trim();
    let mut result = vec![("ascii".to_string(), trimmed.as_bytes().to_vec())];
    if let Some(bytes) = decode_hex_text(trimmed) {
        result.push(("hex".to_string(), bytes));
    }
    if let Ok(value) = parse_number(trimmed) {
        result.push(("u32le".to_string(), (value as u32).to_le_bytes().to_vec()));
        result.push(("u64le".to_string(), value.to_le_bytes().to_vec()));
    }
    result
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn parse_numbers(text: &str) -> Result<Vec<u64>> {
    let values = text
        .split(|char: char| char.is_whitespace() || char == ',')
        .filter(|part| !part.trim().is_empty())
        .map(parse_number)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if values.is_empty() {
        return Err(CtfError::InvalidInput(
            "provide at least one integer value".to_string(),
        ));
    }
    Ok(values)
}

fn parse_number(text: &str) -> Result<u64> {
    let trimmed = text.trim().trim_end_matches(',');
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).map_err(|error| CtfError::InvalidInput(error.to_string()))
    } else {
        trimmed
            .parse::<u64>()
            .map_err(|error| CtfError::InvalidInput(error.to_string()))
    }
}

fn parse_byte_text(text: &str) -> Vec<u8> {
    let trimmed = text.trim();
    if let Some(bytes) = parse_escaped_bytes(trimmed) {
        return bytes;
    }
    if let Some(bytes) = decode_hex_text(trimmed) {
        return bytes;
    }
    trimmed.as_bytes().to_vec()
}

fn parse_escaped_bytes(text: &str) -> Option<Vec<u8>> {
    if !text.contains("\\x") {
        return None;
    }
    let mut bytes = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(char) = chars.next() {
        if char == '\\' && chars.peek() == Some(&'x') {
            chars.next();
            let hi = chars.next()?;
            let lo = chars.next()?;
            let value = u8::from_str_radix(&format!("{hi}{lo}"), 16).ok()?;
            bytes.push(value);
        } else if !char.is_whitespace() {
            bytes.push(char as u8);
        }
    }
    Some(bytes)
}

fn decode_hex_text(text: &str) -> Option<Vec<u8>> {
    let cleaned = text
        .trim()
        .trim_start_matches("0x")
        .replace([' ', '\n', '\r', '\t', '_'], "");
    if cleaned.len() < 2 || !cleaned.len().is_multiple_of(2) {
        return None;
    }
    if !cleaned.chars().all(|char| char.is_ascii_hexdigit()) {
        return None;
    }
    hex::decode(cleaned).ok()
}

fn assemble_x86_64_line(line: &str) -> Result<Vec<u8>> {
    let normalized = line.to_ascii_lowercase().replace('\t', " ");
    let normalized = normalized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" ,", ",")
        .replace(", ", ",");
    let bytes = match normalized.as_str() {
        "nop" => vec![0x90],
        "ret" => vec![0xc3],
        "int3" => vec![0xcc],
        "syscall" => vec![0x0f, 0x05],
        "xor eax,eax" => vec![0x31, 0xc0],
        "xor rax,rax" => vec![0x48, 0x31, 0xc0],
        "xor rdi,rdi" => vec![0x48, 0x31, 0xff],
        "xor rsi,rsi" => vec![0x48, 0x31, 0xf6],
        "xor rdx,rdx" => vec![0x48, 0x31, 0xd2],
        _ if normalized.starts_with("mov al,") => {
            let value = parse_number(&normalized["mov al,".len()..])?;
            if value > u8::MAX as u64 {
                return Err(CtfError::InvalidInput(
                    "mov al immediate exceeds u8".to_string(),
                ));
            }
            vec![0xb0, value as u8]
        }
        _ if normalized.starts_with("push ") => {
            let value = parse_number(&normalized["push ".len()..])?;
            if value <= u8::MAX as u64 {
                vec![0x6a, value as u8]
            } else if value <= u32::MAX as u64 {
                let mut bytes = vec![0x68];
                bytes.extend_from_slice(&(value as u32).to_le_bytes());
                bytes
            } else {
                return Err(CtfError::InvalidInput(
                    "push immediate exceeds u32".to_string(),
                ));
            }
        }
        _ => {
            return Err(CtfError::InvalidInput(format!(
                "unsupported x86_64 assembly line: {line}"
            )));
        }
    };
    Ok(bytes)
}

fn disassemble_x86_64(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let remaining = &bytes[offset..];
        let (size, mnemonic) = if remaining.starts_with(&[0x90]) {
            (1, "nop".to_string())
        } else if remaining.starts_with(&[0xc3]) {
            (1, "ret".to_string())
        } else if remaining.starts_with(&[0xcc]) {
            (1, "int3".to_string())
        } else if remaining.starts_with(&[0x0f, 0x05]) {
            (2, "syscall".to_string())
        } else if remaining.starts_with(&[0x31, 0xc0]) {
            (2, "xor eax, eax".to_string())
        } else if remaining.starts_with(&[0x48, 0x31, 0xc0]) {
            (3, "xor rax, rax".to_string())
        } else if remaining.starts_with(&[0x48, 0x31, 0xff]) {
            (3, "xor rdi, rdi".to_string())
        } else if remaining.starts_with(&[0x48, 0x31, 0xf6]) {
            (3, "xor rsi, rsi".to_string())
        } else if remaining.starts_with(&[0x48, 0x31, 0xd2]) {
            (3, "xor rdx, rdx".to_string())
        } else if remaining.starts_with(&[0xb0]) && remaining.len() >= 2 {
            (2, format!("mov al, 0x{:02x}", remaining[1]))
        } else if remaining.starts_with(&[0x6a]) && remaining.len() >= 2 {
            (2, format!("push 0x{:02x}", remaining[1]))
        } else if remaining.starts_with(&[0x68]) && remaining.len() >= 5 {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&remaining[1..5]);
            (5, format!("push 0x{:08x}", u32::from_le_bytes(buf)))
        } else {
            (1, format!("db 0x{:02x}", remaining[0]))
        };
        let hex_bytes = escaped_bytes(&bytes[offset..offset + size]);
        out.push_str(&format!("{offset:04x}: {hex_bytes:<20} {mnemonic}\n"));
        offset += size;
    }
    out
}

fn strip_comment(line: &str) -> &str {
    line.split([';', '#']).next().unwrap_or(line)
}

fn escaped_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("\\x{byte:02x}"))
        .collect::<String>()
}

fn hex_prefix(bytes: &[u8], max: usize) -> String {
    hex::encode(bytes.iter().take(max).copied().collect::<Vec<_>>())
}

fn read_u16(bytes: &[u8], offset: usize, endian: &str) -> Option<u16> {
    match endian {
        "little" => read_u16_le(bytes, offset),
        "big" => read_u16_be(bytes, offset),
        _ => None,
    }
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_u16_be(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_u32_be(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn elf_machine(machine: u16) -> &'static str {
    match machine {
        0x03 => "x86",
        0x3e => "x86_64",
        0x28 => "ARM",
        0xb7 => "AArch64",
        0xf3 => "RISC-V",
        _ => "unknown",
    }
}

fn pe_machine(machine: u16) -> &'static str {
    match machine {
        0x014c => "x86",
        0x8664 => "x86_64",
        0x01c0 => "ARM",
        0xaa64 => "AArch64",
        _ => "unknown",
    }
}

fn single_output(kind: &str, label: &str, value: String) -> OperationResponse {
    OperationResponse {
        status: "ok".to_string(),
        outputs: vec![OperationOutput {
            kind: kind.to_string(),
            label: label.to_string(),
            value,
        }],
        warnings: vec![],
    }
}

fn json_output(label: &str, value: serde_json::Value) -> Result<OperationResponse> {
    json_response(label, value)
}

fn json_response(label: &str, value: serde_json::Value) -> Result<OperationResponse> {
    Ok(single_output(
        "json",
        label,
        serde_json::to_string_pretty(&value)
            .map_err(|error| CtfError::InvalidInput(error.to_string()))?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(operation: &str, value: &str) -> OperationRequest {
        OperationRequest {
            operation: operation.to_string(),
            input: ctf_core::OperationInput {
                kind: "text".to_string(),
                value: value.to_string(),
            },
            limits: ctf_core::TaskLimits::default(),
        }
    }

    #[test]
    fn cyclic_pattern_is_searchable() {
        let response =
            cyclic_create(&dummy_spec(), &request("pwn.cyclic.create", "64")).expect("cyclic");
        assert_eq!(response.outputs[0].value.len(), 64);

        let needle = &response.outputs[0].value[12..16];
        let offset =
            cyclic_offset(&dummy_spec(), &request("pwn.cyclic.offset", needle)).expect("offset");
        assert!(offset.outputs[0].value.contains("\"offset\": 12"));
    }

    #[test]
    fn pack_and_unpack_u64() {
        let packed =
            pack_u64_le(&dummy_spec(), &request("pwn.pack.u64_le", "0x41424344")).expect("pack");
        assert!(packed.outputs[0].value.contains("4443424100000000"));

        let unpacked = unpack_u64_le(
            &dummy_spec(),
            &request("pwn.unpack.u64_le", "4443424100000000"),
        )
        .expect("unpack");
        assert!(unpacked.outputs[0].value.contains("0x0000000041424344"));
    }

    #[test]
    fn binary_info_detects_elf() {
        let response = binary_info(
            &dummy_spec(),
            &request("binary.info", "\u{7f}ELF\u{2}\u{1}\u{1}"),
        )
        .expect("binary info");
        assert!(response.outputs[0].value.contains("\"format\": \"ELF\""));
    }

    #[test]
    fn shellcode_round_trips_supported_instructions() {
        let assembled = shellcode_asm_x86_64(
            &dummy_spec(),
            &request("shellcode.asm.x86_64", "xor rax, rax\nsyscall"),
        )
        .expect("assemble");
        assert!(assembled.outputs[0].value.contains("4831c00f05"));

        let disassembled = shellcode_disasm_x86_64(
            &dummy_spec(),
            &request("shellcode.disasm.x86_64", "4831c00f05"),
        )
        .expect("disassemble");
        assert!(disassembled.outputs[0].value.contains("xor rax, rax"));
        assert!(disassembled.outputs[0].value.contains("syscall"));
    }

    #[test]
    fn extracts_strings() {
        let response = reverse_strings(
            &dummy_spec(),
            &request("reverse.strings", "\0flag{demo}\0no"),
        )
        .expect("strings");
        assert!(response.outputs[0].value.contains("flag{demo}"));
    }

    fn dummy_spec() -> OperationSpec {
        OperationSpec {
            id: "test".to_string(),
            name_zh: "test".to_string(),
            name_en: "test".to_string(),
            category: "test".to_string(),
            aliases: vec![],
            input: vec!["text".to_string()],
            output: vec!["text".to_string()],
            backend: "rust".to_string(),
            safety: "safe".to_string(),
            deterministic: true,
            batchable: true,
            priority: "P0".to_string(),
            secrets: None,
        }
    }
}
