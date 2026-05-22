//! Text codec implementations.

use base64::Engine;
use ctf_core::{
    CtfError, OperationOutput, OperationRequest, OperationResponse, OperationRunner, OperationSpec,
    Result,
};
use data_encoding::{BASE32, BASE32_NOPAD};
use std::collections::HashSet;

const BASE45_ALPHABET: &[u8; 45] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";
const BASE58_ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const BASE62_ALPHABET: &[u8; 62] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const AUTO_DECODE_MAX_DEPTH: usize = 5;
const AUTO_DECODE_BEAM_WIDTH: usize = 64;
const AUTO_DECODE_MAX_VALUE_LEN: usize = 128 * 1024;
const CTF_SIGNAL_KEYWORDS: &[&str] = &[
    "flag", "ctf", "key", "secret", "token", "password", "passwd", "admin", "root", "shell",
    "upload", "select", "union", "sqlite", "mysql", "crypto", "cipher", "xor", "pwn", "reverse",
    "web", "misc", "forensic",
];
const CTF_CONTEXT_KEYWORDS: &[&str] = &[
    "http://",
    "https://",
    "eyj",
    "{\"",
    "<?php",
    "<script",
    "username",
    "authorization",
];

pub fn register_handlers(runner: &mut OperationRunner) {
    runner.register_handler("base64.decode", base64_decode);
    runner.register_handler("base64.encode", base64_encode);
    runner.register_handler("base64.offsets", base64_offsets);
    runner.register_handler("base32.decode", base32_decode);
    runner.register_handler("base32.encode", base32_encode);
    runner.register_handler("base45.decode", base45_decode);
    runner.register_handler("base45.encode", base45_encode);
    runner.register_handler("base58.decode", base58_decode);
    runner.register_handler("base58.encode", base58_encode);
    runner.register_handler("base62.decode", base62_decode);
    runner.register_handler("base62.encode", base62_encode);
    runner.register_handler("base85.decode", base85_decode);
    runner.register_handler("base85.encode", base85_encode);
    runner.register_handler("url.decode", url_decode);
    runner.register_handler("url.encode", url_encode);
    runner.register_handler("quoted_printable.decode", quoted_printable_decode);
    runner.register_handler("quoted_printable.encode", quoted_printable_encode);
    runner.register_handler("html.decode", html_decode);
    runner.register_handler("html.encode", html_encode);
    runner.register_handler("unicode.decode", unicode_decode);
    runner.register_handler("unicode.encode", unicode_encode);
    runner.register_handler("ascii.decode", ascii_decode);
    runner.register_handler("ascii.encode", ascii_encode);
    runner.register_handler("binary.decode", binary_decode);
    runner.register_handler("binary.encode", binary_encode);
    runner.register_handler("octal.decode", octal_decode);
    runner.register_handler("octal.encode", octal_encode);
    runner.register_handler("decimal.decode", decimal_decode);
    runner.register_handler("decimal.encode", decimal_encode);
    runner.register_handler("hexdump.decode", hexdump_decode);
    runner.register_handler("hexdump.encode", hexdump_encode);
    runner.register_handler("hex.decode", hex_decode);
    runner.register_handler("hex.encode", hex_encode);
    runner.register_handler("endian.swap", endian_swap);
    runner.register_handler("radix.convert", radix_convert);
    runner.register_handler("xor.single_byte_bruteforce", xor_single_byte_bruteforce);
    runner.register_handler("rot13.decode", rot13_decode);
    runner.register_handler("caesar.bruteforce", caesar_bruteforce);
    runner.register_handler("morse.encode", morse_encode);
    runner.register_handler("morse.decode", morse_decode);
    runner.register_handler("brainfuck.run", brainfuck_run);
    runner.register_handler("auto.decode", auto_decode);
}

fn base64_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let text = strip_ascii_ws(&request.input_text()?);
    let bytes = decode_base64_variant(&text)?;
    Ok(single_output("text", "decoded", bytes_to_display(bytes)))
}

fn base64_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let bytes = request.input_bytes()?;
    Ok(single_output(
        "text",
        "base64",
        base64::engine::general_purpose::STANDARD.encode(bytes),
    ))
}

fn base64_offsets(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let text = strip_ascii_ws(&request.input_text()?);
    let offsets = (0..4)
        .filter_map(|offset| {
            let shifted = text.get(offset..)?;
            let usable_len = shifted.len() - shifted.len() % 4;
            if usable_len < 4 {
                return None;
            }
            let candidate = &shifted[..usable_len];
            let decoded = decode_base64_variant(candidate).ok()?;
            Some(serde_json::json!({
                "offset": offset,
                "input_chars": usable_len,
                "decoded_bytes": decoded.len(),
                "preview": preview_decoded_bytes(&decoded),
                "hex": hex::encode(&decoded),
            }))
        })
        .collect::<Vec<_>>();

    json_output(
        "base64-offsets",
        serde_json::json!({
            "input_chars": text.len(),
            "offsets": offsets,
        }),
    )
}

fn base32_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let text = strip_ascii_ws(&request.input_text()?).to_uppercase();
    let bytes = BASE32
        .decode(text.as_bytes())
        .or_else(|_| BASE32_NOPAD.decode(text.trim_end_matches('=').as_bytes()))
        .map_err(|error| CtfError::InvalidInput(format!("invalid base32: {error}")))?;
    Ok(single_output("text", "decoded", bytes_to_display(bytes)))
}

fn base32_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    Ok(single_output(
        "text",
        "base32",
        BASE32.encode(&request.input_bytes()?),
    ))
}

fn base45_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let bytes = decode_base45(&strip_ascii_ws(&request.input_text()?))?;
    Ok(single_output("text", "decoded", bytes_to_display(bytes)))
}

fn base45_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    Ok(single_output(
        "text",
        "base45",
        encode_base45(&request.input_bytes()?),
    ))
}

fn base58_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let bytes = decode_base_n(
        &strip_ascii_ws(&request.input_text()?),
        BASE58_ALPHABET,
        "base58",
    )?;
    Ok(single_output("text", "decoded", bytes_to_display(bytes)))
}

fn base58_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    Ok(single_output(
        "text",
        "base58",
        encode_base_n(&request.input_bytes()?, BASE58_ALPHABET),
    ))
}

fn base62_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let bytes = decode_base_n(
        &strip_ascii_ws(&request.input_text()?),
        BASE62_ALPHABET,
        "base62",
    )?;
    Ok(single_output("text", "decoded", bytes_to_display(bytes)))
}

fn base62_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    Ok(single_output(
        "text",
        "base62",
        encode_base_n(&request.input_bytes()?, BASE62_ALPHABET),
    ))
}

fn base85_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let bytes = decode_ascii85(&request.input_text()?)?;
    Ok(single_output("text", "decoded", bytes_to_display(bytes)))
}

fn base85_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    Ok(single_output(
        "text",
        "base85",
        encode_ascii85(&request.input_bytes()?),
    ))
}

fn url_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let text = request.input_text()?;
    let decoded = urlencoding::decode(&text)
        .map_err(|error| CtfError::InvalidInput(format!("invalid url encoding: {error}")))?;
    Ok(single_output("text", "decoded", decoded.into_owned()))
}

fn url_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    Ok(single_output(
        "text",
        "url",
        urlencoding::encode(&request.input_text()?).into_owned(),
    ))
}

fn quoted_printable_decode(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let bytes = decode_quoted_printable(&request.input_text()?)?;
    Ok(single_output("text", "decoded", bytes_to_display(bytes)))
}

fn quoted_printable_encode(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    Ok(single_output(
        "text",
        "quoted-printable",
        encode_quoted_printable(&request.input_bytes()?),
    ))
}

fn html_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let mut text = request.input_text()?;
    for (from, to) in [
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&apos;", "'"),
        ("&amp;", "&"),
    ] {
        text = text.replace(from, to);
    }
    Ok(single_output("text", "html", text))
}

fn html_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let text = request
        .input_text()?
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;");
    Ok(single_output("text", "html", text))
}

fn unicode_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let text = request.input_text()?;
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && matches!(chars.peek(), Some('u')) {
            chars.next();
            if matches!(chars.peek(), Some('{')) {
                chars.next();
                let mut hex = String::new();
                for next in chars.by_ref() {
                    if next == '}' {
                        break;
                    }
                    hex.push(next);
                }
                push_codepoint(&mut out, &hex)?;
            } else {
                let hex: String = chars.by_ref().take(4).collect();
                push_codepoint(&mut out, &hex)?;
            }
        } else {
            out.push(ch);
        }
    }
    Ok(single_output("text", "unicode", out))
}

fn unicode_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let out = request
        .input_text()?
        .chars()
        .map(|ch| format!("\\u{{{:x}}}", ch as u32))
        .collect::<String>();
    Ok(single_output("text", "unicode", out))
}

fn ascii_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let mut out = String::new();
    for token in request
        .input_text()?
        .split(|ch: char| ch.is_ascii_whitespace() || ch == ',' || ch == ';')
        .filter(|token| !token.is_empty())
    {
        let value = token
            .parse::<u32>()
            .map_err(|_| CtfError::InvalidInput(format!("invalid ascii code: {token}")))?;
        let ch = char::from_u32(value)
            .ok_or_else(|| CtfError::InvalidInput(format!("invalid unicode scalar: {value}")))?;
        out.push(ch);
    }
    Ok(single_output("text", "ascii", out))
}

fn ascii_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let out = request
        .input_text()?
        .chars()
        .map(|ch| (ch as u32).to_string())
        .collect::<Vec<_>>()
        .join(" ");
    Ok(single_output("text", "ascii", out))
}

fn binary_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let bytes = decode_number_bytes(&request.input_text()?, 2, "binary")?;
    Ok(single_output("text", "decoded", bytes_to_display(bytes)))
}

fn binary_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    Ok(single_output(
        "text",
        "binary",
        request
            .input_bytes()?
            .iter()
            .map(|byte| format!("{byte:08b}"))
            .collect::<Vec<_>>()
            .join(" "),
    ))
}

fn octal_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let bytes = decode_number_bytes(&request.input_text()?, 8, "octal")?;
    Ok(single_output("text", "decoded", bytes_to_display(bytes)))
}

fn octal_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    Ok(single_output(
        "text",
        "octal",
        request
            .input_bytes()?
            .iter()
            .map(|byte| format!("{byte:03o}"))
            .collect::<Vec<_>>()
            .join(" "),
    ))
}

fn decimal_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let bytes = decode_number_bytes(&request.input_text()?, 10, "decimal")?;
    Ok(single_output("text", "decoded", bytes_to_display(bytes)))
}

fn decimal_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    Ok(single_output(
        "text",
        "decimal",
        request
            .input_bytes()?
            .iter()
            .map(|byte| byte.to_string())
            .collect::<Vec<_>>()
            .join(" "),
    ))
}

fn hexdump_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let bytes = decode_hexdump(&request.input_text()?)?;
    Ok(single_output("text", "decoded", bytes_to_display(bytes)))
}

fn hexdump_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    Ok(single_output(
        "text",
        "hexdump",
        encode_hexdump(&request.input_bytes()?),
    ))
}

fn hex_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let text = request
        .input_text()?
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect::<String>();
    let bytes = hex::decode(text).map_err(|error| CtfError::InvalidInput(error.to_string()))?;
    Ok(single_output("text", "hex", bytes_to_display(bytes)))
}

fn hex_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    Ok(single_output(
        "text",
        "hex",
        hex::encode(request.input_bytes()?),
    ))
}

fn endian_swap(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let (word_size, bytes) = parse_endian_request(request)?;
    if word_size == 0 || word_size > 64 {
        return Err(CtfError::InvalidInput(
            "word size must be between 1 and 64".to_string(),
        ));
    }

    let mut out = Vec::with_capacity(bytes.len());
    for chunk in bytes.chunks(word_size) {
        out.extend(chunk.iter().rev());
    }
    Ok(single_output("text", "swapped-hex", hex::encode(out)))
}

fn radix_convert(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let parsed = parse_radix_request(&request.input_text()?)?;
    let number = u128::from_str_radix(&parsed.value, parsed.from).map_err(|error| {
        CtfError::InvalidInput(format!("invalid base{} integer: {error}", parsed.from))
    })?;
    let value = serde_json::json!({
        "input": parsed.value,
        "from": parsed.from,
        "to": parsed.to,
        "converted": format_radix(number, parsed.to),
        "base2": format_radix(number, 2),
        "base8": format_radix(number, 8),
        "base10": number.to_string(),
        "base16": format_radix(number, 16),
    });
    json_output("radix", value)
}

fn xor_single_byte_bruteforce(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let bytes = if request.input.kind == "text" {
        parse_hex_or_raw(&request.input.value)
    } else {
        request.input_bytes()?
    };
    if bytes.is_empty() {
        return Err(CtfError::InvalidInput("input is empty".to_string()));
    }

    let mut candidates = (0u8..=255)
        .map(|key| {
            let decoded = bytes.iter().map(|byte| byte ^ key).collect::<Vec<_>>();
            let text = bytes_to_lossy_text(&decoded);
            let score = score_xor_plaintext(&decoded, &text);
            serde_json::json!({
                "key": key,
                "key_hex": format!("0x{key:02x}"),
                "key_ascii": if key.is_ascii_graphic() { (key as char).to_string() } else { String::new() },
                "score": score,
                "text": text,
                "hex": hex::encode(&decoded),
            })
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|a, b| {
        b["score"]
            .as_f64()
            .unwrap_or_default()
            .total_cmp(&a["score"].as_f64().unwrap_or_default())
    });
    candidates.truncate(16);
    json_output(
        "xor-bruteforce",
        serde_json::json!({
            "candidates": candidates,
        }),
    )
}

fn rot13_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    Ok(single_output(
        "text",
        "rot13",
        request.input_text()?.chars().map(rot13_char).collect(),
    ))
}

fn caesar_bruteforce(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let input = request.input_text()?;
    let candidates = (0u8..26)
        .map(|shift| {
            let text = input
                .chars()
                .map(|ch| caesar_shift_char(ch, 26 - shift))
                .collect::<String>();
            serde_json::json!({
                "shift": shift,
                "score": score_text(&text),
                "text": text,
            })
        })
        .collect::<Vec<_>>();
    let mut candidates = candidates;
    candidates.sort_by(|a, b| {
        b["score"]
            .as_f64()
            .unwrap_or_default()
            .total_cmp(&a["score"].as_f64().unwrap_or_default())
    });
    json_output(
        "caesar",
        serde_json::json!({
            "candidates": candidates,
        }),
    )
}

fn morse_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let mut words = Vec::new();
    for word in request.input_text()?.split_whitespace() {
        let letters = word
            .chars()
            .map(|ch| {
                morse_for_char(ch).ok_or_else(|| {
                    CtfError::InvalidInput(format!("unsupported morse character: {ch}"))
                })
            })
            .collect::<Result<Vec<_>>>()?;
        words.push(letters.join(" "));
    }
    Ok(single_output("text", "morse", words.join(" / ")))
}

fn morse_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let input = request.input_text()?;
    let mut output = String::new();
    for token in input.split_whitespace() {
        if token == "/" || token == "|" {
            output.push(' ');
        } else if let Some(ch) = char_for_morse(token) {
            output.push(ch);
        } else {
            return Err(CtfError::InvalidInput(format!(
                "unsupported morse token: {token}"
            )));
        }
    }
    Ok(single_output("text", "morse", output))
}

fn brainfuck_run(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let program = request.input_text()?;
    let output = run_brainfuck(&program, request.limits.timeout_ms)?;
    Ok(single_output("text", "brainfuck", output))
}

fn auto_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let input = request.input_text()?;
    let candidates = search_auto_decode_paths(&input);
    let value = format_auto_decode_candidates(&candidates);
    Ok(single_output("text", "auto", value))
}

fn push_codepoint(out: &mut String, hex: &str) -> Result<()> {
    let value = u32::from_str_radix(hex, 16)
        .map_err(|_| CtfError::InvalidInput(format!("invalid unicode escape: {hex}")))?;
    let ch = char::from_u32(value)
        .ok_or_else(|| CtfError::InvalidInput(format!("invalid unicode scalar: {value}")))?;
    out.push(ch);
    Ok(())
}

fn strip_ascii_ws(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect()
}

fn decode_base64_variant(text: &str) -> Result<Vec<u8>> {
    let padded = match text.len() % 4 {
        0 => text.to_string(),
        2 => format!("{text}=="),
        3 => format!("{text}="),
        _ => text.to_string(),
    };
    base64::engine::general_purpose::STANDARD
        .decode(text.as_bytes())
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(text.as_bytes()))
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(padded.as_bytes()))
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(padded.as_bytes()))
        .map_err(|error| CtfError::InvalidInput(format!("invalid base64: {error}")))
}

fn preview_decoded_bytes(bytes: &[u8]) -> String {
    let preview = bytes_to_lossy_text(bytes);
    preview_text_value(&preview, 96)
}

fn preview_text_value(value: &str, limit: usize) -> String {
    let mut out = String::new();
    for (index, ch) in value.replace(['\r', '\n'], " ").chars().enumerate() {
        if index >= limit {
            out.push_str("...");
            return out;
        }
        out.push(ch);
    }
    out
}

fn encode_hexdump(bytes: &[u8]) -> String {
    let mut lines = Vec::new();
    for (offset, chunk) in bytes.chunks(16).enumerate() {
        let address = offset * 16;
        let mut hex_part = String::new();
        for index in 0..16 {
            if index > 0 {
                hex_part.push(' ');
                if index == 8 {
                    hex_part.push(' ');
                }
            }
            if let Some(byte) = chunk.get(index) {
                hex_part.push_str(&format!("{byte:02x}"));
            } else {
                hex_part.push_str("  ");
            }
        }
        let ascii = chunk
            .iter()
            .map(|byte| {
                if byte.is_ascii_graphic() || *byte == b' ' {
                    *byte as char
                } else {
                    '.'
                }
            })
            .collect::<String>();
        lines.push(format!("{address:08x}  {hex_part}  |{ascii}|"));
    }
    lines.join("\n")
}

fn decode_hexdump(text: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    for line in text.lines() {
        let hex_part = line.split('|').next().unwrap_or(line);
        let mut tokens = hex_part.split_whitespace().collect::<Vec<_>>();
        if tokens.first().is_some_and(|token| {
            let trimmed = token.trim_end_matches(':');
            (tokens.len() > 1 || token.ends_with(':') || line.contains('|'))
                && is_hexdump_offset(trimmed)
        }) {
            tokens.remove(0);
        }

        for token in tokens {
            let token = token.trim_matches(|ch: char| ch == ':' || ch == ';' || ch == ',');
            if token.len() == 2 && token.chars().all(|ch| ch.is_ascii_hexdigit()) {
                let value = u8::from_str_radix(token, 16).map_err(|error| {
                    CtfError::InvalidInput(format!("invalid hexdump byte `{token}`: {error}"))
                })?;
                bytes.push(value);
            } else if token.len() > 2
                && token.len() % 2 == 0
                && token.chars().all(|ch| ch.is_ascii_hexdigit())
                && !line.contains('|')
            {
                for index in (0..token.len()).step_by(2) {
                    let value =
                        u8::from_str_radix(&token[index..index + 2], 16).map_err(|error| {
                            CtfError::InvalidInput(format!(
                                "invalid hexdump byte `{}`: {error}",
                                &token[index..index + 2]
                            ))
                        })?;
                    bytes.push(value);
                }
            }
        }
    }

    if bytes.is_empty() {
        return Err(CtfError::InvalidInput(
            "no hexadecimal bytes found in hexdump".to_string(),
        ));
    }
    Ok(bytes)
}

fn is_hexdump_offset(token: &str) -> bool {
    (4..=16).contains(&token.len()) && token.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn parse_endian_request(request: &OperationRequest) -> Result<(usize, Vec<u8>)> {
    if request.input.kind != "text" {
        return Ok((4, request.input_bytes()?));
    }

    let text = request.input_text()?;
    let mut word_size = 4usize;
    let mut value_parts = Vec::new();
    for token in text.split_whitespace() {
        if let Some(value) = token
            .strip_prefix("word=")
            .or_else(|| token.strip_prefix("size="))
            .or_else(|| token.strip_prefix("chunk="))
        {
            word_size = value.parse::<usize>().map_err(|error| {
                CtfError::InvalidInput(format!("invalid endian word size: {error}"))
            })?;
        } else if let Some(value) = token.strip_prefix("value=") {
            value_parts.push(value.to_string());
        } else {
            value_parts.push(token.to_string());
        }
    }

    let value = value_parts.join("");
    let bytes = if value.chars().all(|ch| ch.is_ascii_hexdigit()) && value.len() % 2 == 0 {
        hex::decode(&value).map_err(|error| CtfError::InvalidInput(error.to_string()))?
    } else {
        text.as_bytes().to_vec()
    };
    Ok((word_size, bytes))
}

fn encode_base45(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(2) {
        if chunk.len() == 2 {
            let value = u16::from(chunk[0]) * 256 + u16::from(chunk[1]);
            out.push(BASE45_ALPHABET[usize::from(value % 45)] as char);
            out.push(BASE45_ALPHABET[usize::from((value / 45) % 45)] as char);
            out.push(BASE45_ALPHABET[usize::from(value / (45 * 45))] as char);
        } else {
            let value = u16::from(chunk[0]);
            out.push(BASE45_ALPHABET[usize::from(value % 45)] as char);
            out.push(BASE45_ALPHABET[usize::from(value / 45)] as char);
        }
    }
    out
}

fn decode_base45(text: &str) -> Result<Vec<u8>> {
    let values = text
        .bytes()
        .map(|byte| {
            BASE45_ALPHABET
                .iter()
                .position(|candidate| *candidate == byte)
                .map(|value| value as u32)
                .ok_or_else(|| CtfError::InvalidInput(format!("invalid base45 character: {byte}")))
        })
        .collect::<Result<Vec<_>>>()?;

    let mut out = Vec::new();
    let mut chunks = values.chunks_exact(3);
    for chunk in &mut chunks {
        let value = chunk[0] + chunk[1] * 45 + chunk[2] * 45 * 45;
        if value > 0xffff {
            return Err(CtfError::InvalidInput("invalid base45 triplet".to_string()));
        }
        out.push((value / 256) as u8);
        out.push((value % 256) as u8);
    }

    match chunks.remainder() {
        [] => {}
        [first, second] => {
            let value = first + second * 45;
            if value > 0xff {
                return Err(CtfError::InvalidInput("invalid base45 pair".to_string()));
            }
            out.push(value as u8);
        }
        [_] => {
            return Err(CtfError::InvalidInput(
                "base45 input cannot have length 1 modulo 3".to_string(),
            ));
        }
        _ => unreachable!(),
    }
    Ok(out)
}

fn encode_base_n(bytes: &[u8], alphabet: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let base = alphabet.len() as u32;
    let mut digits: Vec<u32> = vec![0];
    for byte in bytes {
        let mut carry = u32::from(*byte);
        for digit in digits.iter_mut().rev() {
            let value = *digit * 256 + carry;
            *digit = value % base;
            carry = value / base;
        }
        while carry > 0 {
            digits.insert(0, carry % base);
            carry /= base;
        }
    }

    let leading_zeroes = bytes.iter().take_while(|byte| **byte == 0).count();
    let mut out = String::new();
    for _ in 0..leading_zeroes {
        out.push(alphabet[0] as char);
    }
    let first_non_zero = digits
        .iter()
        .position(|digit| *digit != 0)
        .unwrap_or(digits.len());
    for digit in &digits[first_non_zero..] {
        out.push(alphabet[*digit as usize] as char);
    }
    if out.is_empty() {
        out.push(alphabet[0] as char);
    }
    out
}

fn decode_base_n(text: &str, alphabet: &[u8], name: &str) -> Result<Vec<u8>> {
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let base = alphabet.len() as u32;
    let mut bytes: Vec<u32> = vec![0];
    for byte in text.bytes() {
        let Some(mut carry) = alphabet
            .iter()
            .position(|candidate| *candidate == byte)
            .map(|value| value as u32)
        else {
            return Err(CtfError::InvalidInput(format!(
                "invalid {name} character: {byte}"
            )));
        };
        for item in bytes.iter_mut().rev() {
            let value = *item * base + carry;
            *item = value & 0xff;
            carry = value >> 8;
        }
        while carry > 0 {
            bytes.insert(0, carry & 0xff);
            carry >>= 8;
        }
    }

    let leading_zeroes = text.bytes().take_while(|byte| *byte == alphabet[0]).count();
    let first_non_zero = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len());
    let mut out = vec![0; leading_zeroes];
    out.extend(bytes[first_non_zero..].iter().map(|byte| *byte as u8));
    Ok(out)
}

fn encode_ascii85(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(4) {
        let mut block = [0u8; 4];
        block[..chunk.len()].copy_from_slice(chunk);
        let mut value = u32::from_be_bytes(block);
        let mut encoded = [0u8; 5];
        for item in encoded.iter_mut().rev() {
            *item = (value % 85) as u8 + 33;
            value /= 85;
        }
        let length = if chunk.len() == 4 { 5 } else { chunk.len() + 1 };
        for byte in &encoded[..length] {
            out.push(*byte as char);
        }
    }
    out
}

fn decode_ascii85(text: &str) -> Result<Vec<u8>> {
    let normalized = text
        .trim()
        .trim_start_matches("<~")
        .trim_end_matches("~>")
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect::<String>();
    let mut out = Vec::new();
    let mut chunk = Vec::new();

    for ch in normalized.bytes() {
        if ch == b'z' && chunk.is_empty() {
            out.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        if !(33..=117).contains(&ch) {
            return Err(CtfError::InvalidInput(format!(
                "invalid base85 character: {ch}"
            )));
        }
        chunk.push(ch - 33);
        if chunk.len() == 5 {
            out.extend_from_slice(&decode_ascii85_chunk(&chunk, 4)?);
            chunk.clear();
        }
    }

    if !chunk.is_empty() {
        if chunk.len() == 1 {
            return Err(CtfError::InvalidInput(
                "base85 trailing chunk is too short".to_string(),
            ));
        }
        let output_len = chunk.len() - 1;
        while chunk.len() < 5 {
            chunk.push(84);
        }
        out.extend_from_slice(&decode_ascii85_chunk(&chunk, output_len)?);
    }

    Ok(out)
}

fn decode_ascii85_chunk(chunk: &[u8], output_len: usize) -> Result<Vec<u8>> {
    let mut value = 0u32;
    for digit in chunk {
        value = value
            .checked_mul(85)
            .and_then(|value| value.checked_add(u32::from(*digit)))
            .ok_or_else(|| CtfError::InvalidInput("base85 value overflow".to_string()))?;
    }
    Ok(value.to_be_bytes()[..output_len].to_vec())
}

fn decode_number_bytes(text: &str, radix: u32, name: &str) -> Result<Vec<u8>> {
    text.split(|ch: char| ch.is_ascii_whitespace() || ch == ',' || ch == ';')
        .filter(|token| !token.is_empty())
        .map(|token| {
            u8::from_str_radix(token, radix)
                .map_err(|error| CtfError::InvalidInput(format!("invalid {name} byte: {error}")))
        })
        .collect()
}

fn encode_quoted_printable(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (index, byte) in bytes.iter().enumerate() {
        let next = bytes.get(index + 1).copied();
        match *byte {
            b'\r' => out.push('\r'),
            b'\n' => out.push('\n'),
            b'\t' | b' ' if matches!(next, Some(b'\r') | Some(b'\n') | None) => {
                out.push_str(&format!("={byte:02X}"));
            }
            b'\t' | b' ' => out.push(*byte as char),
            33..=60 | 62..=126 => out.push(*byte as char),
            _ => out.push_str(&format!("={byte:02X}")),
        }
    }
    out
}

fn decode_quoted_printable(text: &str) -> Result<Vec<u8>> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'=' {
            out.push(bytes[index]);
            index += 1;
            continue;
        }

        match bytes.get(index + 1) {
            Some(b'\n') => index += 2,
            Some(b'\r') if bytes.get(index + 2) == Some(&b'\n') => index += 3,
            Some(first) if index + 2 < bytes.len() => {
                let second = bytes[index + 2];
                let hex = [*first, second];
                let hex = std::str::from_utf8(&hex)
                    .map_err(|_| CtfError::InvalidInput("invalid quoted-printable".to_string()))?;
                let value = u8::from_str_radix(hex, 16).map_err(|_| {
                    CtfError::InvalidInput(format!("invalid quoted-printable escape: ={hex}"))
                })?;
                out.push(value);
                index += 3;
            }
            _ => {
                return Err(CtfError::InvalidInput(
                    "truncated quoted-printable escape".to_string(),
                ));
            }
        }
    }
    Ok(out)
}

fn bytes_to_display(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap_or_else(|error| hex::encode(error.into_bytes()))
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
    Ok(single_output(
        "json",
        label,
        serde_json::to_string_pretty(&value)
            .map_err(|error| CtfError::InvalidInput(error.to_string()))?,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RadixRequest {
    from: u32,
    to: u32,
    value: String,
}

fn parse_radix_request(text: &str) -> Result<RadixRequest> {
    let mut from = None;
    let mut to = None;
    let mut value = None;
    let mut positional = Vec::new();

    for token in text
        .split(|ch: char| ch.is_ascii_whitespace() || ch == ',' || ch == ';')
        .filter(|token| !token.is_empty())
    {
        if let Some((key, item)) = token.split_once('=') {
            match key.trim().to_ascii_lowercase().as_str() {
                "from" | "src" | "in" => from = Some(parse_base(item)?),
                "to" | "dst" | "out" => to = Some(parse_base(item)?),
                "value" | "n" | "num" => value = Some(normalize_radix_digits(item)),
                _ => positional.push(token.to_string()),
            }
        } else {
            positional.push(token.to_string());
        }
    }

    if from.is_none() && positional.len() >= 3 {
        from = Some(parse_base(&positional[0])?);
        to = Some(parse_base(&positional[1])?);
        value = Some(normalize_radix_digits(&positional[2..].join("")));
    }

    let from = from.unwrap_or(10);
    let to = to.unwrap_or(16);
    validate_base(from)?;
    validate_base(to)?;
    let value = value
        .or_else(|| positional.first().map(|item| normalize_radix_digits(item)))
        .ok_or_else(|| {
            CtfError::InvalidInput(
                "provide value with `from=16 to=10 value=ff` or `16 10 ff`".to_string(),
            )
        })?;

    Ok(RadixRequest { from, to, value })
}

fn parse_base(value: &str) -> Result<u32> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|error| CtfError::InvalidInput(format!("invalid base: {error}")))
}

fn validate_base(base: u32) -> Result<()> {
    if !(2..=36).contains(&base) {
        return Err(CtfError::InvalidInput(format!(
            "base must be between 2 and 36, got {base}"
        )));
    }
    Ok(())
}

fn normalize_radix_digits(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X")
        .replace('_', "")
}

fn format_radix(mut value: u128, base: u32) -> String {
    debug_assert!((2..=36).contains(&base));
    if value == 0 {
        return "0".to_string();
    }
    let mut digits = Vec::new();
    while value > 0 {
        let digit = (value % base as u128) as u8;
        let ch = match digit {
            0..=9 => (b'0' + digit) as char,
            _ => (b'a' + digit - 10) as char,
        };
        digits.push(ch);
        value /= base as u128;
    }
    digits.iter().rev().collect()
}

fn parse_hex_or_raw(text: &str) -> Vec<u8> {
    let cleaned = text
        .trim()
        .trim_start_matches("0x")
        .replace([' ', '\n', '\r', '\t', ':', '-'], "");
    if cleaned.len() >= 2
        && cleaned.len().is_multiple_of(2)
        && cleaned.chars().all(|ch| ch.is_ascii_hexdigit())
        && let Ok(bytes) = hex::decode(cleaned)
    {
        return bytes;
    }
    text.as_bytes().to_vec()
}

fn bytes_to_lossy_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn rot13_char(ch: char) -> char {
    match ch {
        'a'..='z' => (((ch as u8 - b'a' + 13) % 26) + b'a') as char,
        'A'..='Z' => (((ch as u8 - b'A' + 13) % 26) + b'A') as char,
        _ => ch,
    }
}

fn caesar_shift_char(ch: char, shift: u8) -> char {
    match ch {
        'a'..='z' => (((ch as u8 - b'a' + shift) % 26) + b'a') as char,
        'A'..='Z' => (((ch as u8 - b'A' + shift) % 26) + b'A') as char,
        _ => ch,
    }
}

fn morse_for_char(ch: char) -> Option<&'static str> {
    Some(match ch.to_ascii_uppercase() {
        'A' => ".-",
        'B' => "-...",
        'C' => "-.-.",
        'D' => "-..",
        'E' => ".",
        'F' => "..-.",
        'G' => "--.",
        'H' => "....",
        'I' => "..",
        'J' => ".---",
        'K' => "-.-",
        'L' => ".-..",
        'M' => "--",
        'N' => "-.",
        'O' => "---",
        'P' => ".--.",
        'Q' => "--.-",
        'R' => ".-.",
        'S' => "...",
        'T' => "-",
        'U' => "..-",
        'V' => "...-",
        'W' => ".--",
        'X' => "-..-",
        'Y' => "-.--",
        'Z' => "--..",
        '0' => "-----",
        '1' => ".----",
        '2' => "..---",
        '3' => "...--",
        '4' => "....-",
        '5' => ".....",
        '6' => "-....",
        '7' => "--...",
        '8' => "---..",
        '9' => "----.",
        _ => return None,
    })
}

fn char_for_morse(token: &str) -> Option<char> {
    let table = [
        (".-", 'A'),
        ("-...", 'B'),
        ("-.-.", 'C'),
        ("-..", 'D'),
        (".", 'E'),
        ("..-.", 'F'),
        ("--.", 'G'),
        ("....", 'H'),
        ("..", 'I'),
        (".---", 'J'),
        ("-.-", 'K'),
        (".-..", 'L'),
        ("--", 'M'),
        ("-.", 'N'),
        ("---", 'O'),
        (".--.", 'P'),
        ("--.-", 'Q'),
        (".-.", 'R'),
        ("...", 'S'),
        ("-", 'T'),
        ("..-", 'U'),
        ("...-", 'V'),
        (".--", 'W'),
        ("-..-", 'X'),
        ("-.--", 'Y'),
        ("--..", 'Z'),
        ("-----", '0'),
        (".----", '1'),
        ("..---", '2'),
        ("...--", '3'),
        ("....-", '4'),
        (".....", '5'),
        ("-....", '6'),
        ("--...", '7'),
        ("---..", '8'),
        ("----.", '9'),
    ];
    table
        .iter()
        .find(|(morse, _)| *morse == token)
        .map(|(_, ch)| *ch)
}

fn run_brainfuck(program: &str, timeout_ms: u64) -> Result<String> {
    let instructions = program
        .chars()
        .filter(|ch| matches!(ch, '>' | '<' | '+' | '-' | '.' | ',' | '[' | ']'))
        .collect::<Vec<_>>();
    let jumps = brainfuck_jump_table(&instructions)?;
    let mut tape = vec![0u8; 30_000];
    let mut data_ptr = 0usize;
    let mut inst_ptr = 0usize;
    let mut steps = 0u64;
    let max_steps = timeout_ms.saturating_mul(1_000).max(100_000);
    let mut output = Vec::new();

    while inst_ptr < instructions.len() {
        steps += 1;
        if steps > max_steps {
            return Err(CtfError::InvalidInput(format!(
                "brainfuck step limit exceeded ({max_steps})"
            )));
        }

        match instructions[inst_ptr] {
            '>' => {
                data_ptr += 1;
                if data_ptr == tape.len() {
                    tape.push(0);
                }
            }
            '<' => {
                data_ptr = data_ptr.checked_sub(1).ok_or_else(|| {
                    CtfError::InvalidInput("brainfuck pointer moved before tape start".to_string())
                })?;
            }
            '+' => tape[data_ptr] = tape[data_ptr].wrapping_add(1),
            '-' => tape[data_ptr] = tape[data_ptr].wrapping_sub(1),
            '.' => output.push(tape[data_ptr]),
            ',' => tape[data_ptr] = 0,
            '[' if tape[data_ptr] == 0 => inst_ptr = jumps[&inst_ptr],
            ']' if tape[data_ptr] != 0 => inst_ptr = jumps[&inst_ptr],
            _ => {}
        }
        inst_ptr += 1;
    }

    Ok(bytes_to_display(output))
}

fn brainfuck_jump_table(instructions: &[char]) -> Result<std::collections::HashMap<usize, usize>> {
    let mut stack = Vec::new();
    let mut jumps = std::collections::HashMap::new();
    for (index, instruction) in instructions.iter().enumerate() {
        match instruction {
            '[' => stack.push(index),
            ']' => {
                let Some(open) = stack.pop() else {
                    return Err(CtfError::InvalidInput(
                        "unmatched brainfuck `]`".to_string(),
                    ));
                };
                jumps.insert(open, index);
                jumps.insert(index, open);
            }
            _ => {}
        }
    }
    if !stack.is_empty() {
        return Err(CtfError::InvalidInput(
            "unmatched brainfuck `[`".to_string(),
        ));
    }
    Ok(jumps)
}

fn score_xor_plaintext(bytes: &[u8], text: &str) -> f64 {
    let printable = bytes
        .iter()
        .filter(|byte| byte.is_ascii_graphic() || byte.is_ascii_whitespace())
        .count() as f64
        / bytes.len().max(1) as f64;
    let mut score = printable;
    let lower = text.to_ascii_lowercase();
    for marker in [
        "flag{", "ctf{", "http", "password", "admin", " the ", " and ",
    ] {
        if lower.contains(marker) {
            score += 0.5;
        }
    }
    score
}

type AutoDecodeHandler = fn(&OperationSpec, &OperationRequest) -> Result<OperationResponse>;
type AutoDecodeGate = fn(&str) -> bool;

struct AutoDecoder {
    id: &'static str,
    handler: AutoDecodeHandler,
    gate: AutoDecodeGate,
}

#[derive(Clone)]
struct AutoSearchState {
    path: Vec<&'static str>,
    score: f32,
    value: String,
}

struct AutoCandidate {
    path: String,
    score: f32,
    value: String,
}

fn search_auto_decode_paths(input: &str) -> Vec<AutoCandidate> {
    let decoders = auto_decoders();
    let mut visited = HashSet::from([input.to_string()]);
    let mut frontier = vec![AutoSearchState {
        path: Vec::new(),
        score: score_auto_value(input, 0),
        value: input.to_string(),
    }];
    let mut candidates = Vec::new();

    for _ in 0..AUTO_DECODE_MAX_DEPTH {
        let mut next_frontier = Vec::new();

        for state in &frontier {
            for decoder in &decoders {
                if !should_try_decoder(&state.path, decoder, &state.value) {
                    continue;
                }
                let Some(decoded) = run_auto_decoder(decoder, &state.value) else {
                    continue;
                };
                if decoded == state.value
                    || decoded.trim().is_empty()
                    || decoded.len() > AUTO_DECODE_MAX_VALUE_LEN
                    || !visited.insert(decoded.clone())
                {
                    continue;
                }

                let mut path = state.path.clone();
                path.push(decoder.id);
                let score = score_auto_value(&decoded, path.len());
                candidates.push(AutoCandidate {
                    path: path.join(" -> "),
                    score,
                    value: decoded.clone(),
                });

                next_frontier.push(AutoSearchState {
                    path,
                    score,
                    value: decoded,
                });
            }
        }

        if next_frontier.is_empty() {
            break;
        }
        next_frontier.sort_by(|a, b| b.score.total_cmp(&a.score));
        next_frontier.truncate(AUTO_DECODE_BEAM_WIDTH);
        frontier = next_frontier;
    }

    candidates.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.path.len().cmp(&b.path.len()))
    });
    dedupe_auto_candidates(candidates)
}

fn auto_decoders() -> Vec<AutoDecoder> {
    vec![
        AutoDecoder {
            id: "base64.decode",
            handler: base64_decode,
            gate: looks_like_base64_text,
        },
        AutoDecoder {
            id: "base32.decode",
            handler: base32_decode,
            gate: looks_like_base32_text,
        },
        AutoDecoder {
            id: "base45.decode",
            handler: base45_decode,
            gate: looks_like_base45_text,
        },
        AutoDecoder {
            id: "base58.decode",
            handler: base58_decode,
            gate: looks_like_base58_text,
        },
        AutoDecoder {
            id: "base62.decode",
            handler: base62_decode,
            gate: looks_like_base62_text,
        },
        AutoDecoder {
            id: "base85.decode",
            handler: base85_decode,
            gate: looks_like_base85_text,
        },
        AutoDecoder {
            id: "url.decode",
            handler: url_decode,
            gate: looks_like_url_encoded_text,
        },
        AutoDecoder {
            id: "html.decode",
            handler: html_decode,
            gate: looks_like_html_encoded_text,
        },
        AutoDecoder {
            id: "unicode.decode",
            handler: unicode_decode,
            gate: looks_like_unicode_encoded_text,
        },
        AutoDecoder {
            id: "quoted_printable.decode",
            handler: quoted_printable_decode,
            gate: looks_like_quoted_printable_text,
        },
        AutoDecoder {
            id: "hex.decode",
            handler: hex_decode,
            gate: looks_like_hex_text,
        },
        AutoDecoder {
            id: "binary.decode",
            handler: binary_decode,
            gate: looks_like_binary_text,
        },
        AutoDecoder {
            id: "octal.decode",
            handler: octal_decode,
            gate: looks_like_octal_text,
        },
        AutoDecoder {
            id: "decimal.decode",
            handler: decimal_decode,
            gate: looks_like_decimal_byte_text,
        },
        AutoDecoder {
            id: "ascii.decode",
            handler: ascii_decode,
            gate: looks_like_ascii_code_text,
        },
        AutoDecoder {
            id: "rot13.decode",
            handler: rot13_decode,
            gate: looks_like_rot13_text,
        },
        AutoDecoder {
            id: "morse.decode",
            handler: morse_decode,
            gate: looks_like_morse_text,
        },
    ]
}

fn should_try_decoder(path: &[&str], decoder: &AutoDecoder, value: &str) -> bool {
    if path.last().is_some_and(|last| *last == decoder.id) {
        return false;
    }
    if matches!(decoder.id, "base58.decode" | "base62.decode") && looks_like_hex_text(value) {
        return false;
    }
    (decoder.gate)(value)
}

fn run_auto_decoder(decoder: &AutoDecoder, value: &str) -> Option<String> {
    let request = OperationRequest {
        operation: decoder.id.to_string(),
        input: ctf_core::OperationInput {
            kind: "text".to_string(),
            value: value.to_string(),
        },
        limits: ctf_core::TaskLimits::default(),
    };
    let response = (decoder.handler)(&dummy_operation(decoder.id), &request).ok()?;
    response.outputs.first().map(|output| output.value.clone())
}

fn dedupe_auto_candidates(candidates: Vec<AutoCandidate>) -> Vec<AutoCandidate> {
    let mut seen_values = HashSet::new();
    candidates
        .into_iter()
        .filter(|candidate| seen_values.insert(candidate.value.clone()))
        .collect()
}

fn score_auto_value(value: &str, depth: usize) -> f32 {
    score_text(value) + (depth as f32 * 0.04)
}

fn score_text(value: &str) -> f32 {
    let printable = value
        .chars()
        .filter(|ch| !ch.is_control() || ch.is_ascii_whitespace())
        .count() as f32;
    let total = value.chars().count().max(1) as f32;
    let mut score = printable / total;
    let lower = value.to_ascii_lowercase();
    if lower.contains("flag{") || lower.contains("ctf{") {
        score += 2.0;
    }
    score += ctf_keyword_score(&lower);
    score += brace_pattern_score(value);
    for marker in CTF_CONTEXT_KEYWORDS {
        if lower.contains(marker) {
            score += 0.2;
        }
    }
    if looks_like_encoded_blob(value) {
        score -= 0.15;
    }
    score
}

fn ctf_keyword_score(lower: &str) -> f32 {
    let hits = CTF_SIGNAL_KEYWORDS
        .iter()
        .filter(|keyword| lower.contains(**keyword))
        .count();
    (hits as f32 * 0.18).min(1.2)
}

fn brace_pattern_score(value: &str) -> f32 {
    let mut score: f32 = 0.0;
    for (open, close) in [('{', '}'), ('[', ']'), ('(', ')')] {
        if let Some(candidate) = first_wrapped_payload(value, open, close) {
            score = score.max(if candidate_has_ctf_shape(candidate) {
                1.0
            } else {
                0.45
            });
        }
    }
    score
}

fn first_wrapped_payload(value: &str, open: char, close: char) -> Option<&str> {
    let start = value.find(open)?;
    let tail = &value[start + open.len_utf8()..];
    let end = tail.find(close)?;
    let payload = &tail[..end];
    if (3..=128).contains(&payload.chars().count()) {
        Some(payload)
    } else {
        None
    }
}

fn candidate_has_ctf_shape(value: &str) -> bool {
    value.chars().all(|ch| {
        ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':' | '@' | '/' | '=' | '+')
    })
}

fn looks_like_encoded_blob(value: &str) -> bool {
    let compact = strip_ascii_ws(value);
    compact.len() >= 16
        && (looks_like_base64_text(value)
            || looks_like_base32_text(value)
            || looks_like_hex_text(value)
            || looks_like_url_encoded_text(value))
}

fn looks_like_base64_text(value: &str) -> bool {
    let compact = strip_ascii_ws(value);
    compact.len() >= 4
        && compact.len() % 4 != 1
        && compact
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '/' | '-' | '_' | '='))
}

fn looks_like_base32_text(value: &str) -> bool {
    let compact = strip_ascii_ws(value).to_ascii_uppercase();
    compact.len() >= 8
        && compact.len() % 8 != 1
        && compact
            .chars()
            .all(|ch| matches!(ch, 'A'..='Z' | '2'..='7' | '='))
}

fn looks_like_base45_text(value: &str) -> bool {
    let compact = strip_ascii_ws(value);
    compact.len() >= 6 && compact.bytes().all(|byte| BASE45_ALPHABET.contains(&byte))
}

fn looks_like_base58_text(value: &str) -> bool {
    let compact = strip_ascii_ws(value);
    compact.len() >= 8 && compact.bytes().all(|byte| BASE58_ALPHABET.contains(&byte))
}

fn looks_like_base62_text(value: &str) -> bool {
    let compact = strip_ascii_ws(value);
    compact.len() >= 8
        && compact.chars().all(|ch| ch.is_ascii_alphanumeric())
        && compact.chars().any(|ch| ch.is_ascii_digit())
        && compact.chars().any(|ch| ch.is_ascii_alphabetic())
}

fn looks_like_base85_text(value: &str) -> bool {
    let compact = value.trim();
    compact.len() >= 5
        && compact.bytes().all(|byte| (33..=117).contains(&byte))
        && (compact.contains("<~")
            || compact.chars().any(|ch| {
                !ch.is_ascii_alphanumeric() && !matches!(ch, '+' | '/' | '-' | '_' | '=')
            }))
}

fn looks_like_url_encoded_text(value: &str) -> bool {
    value.as_bytes().windows(3).any(|window| {
        window[0] == b'%' && window[1].is_ascii_hexdigit() && window[2].is_ascii_hexdigit()
    })
}

fn looks_like_html_encoded_text(value: &str) -> bool {
    ["&lt;", "&gt;", "&quot;", "&#39;", "&apos;", "&amp;"]
        .iter()
        .any(|entity| value.contains(entity))
}

fn looks_like_unicode_encoded_text(value: &str) -> bool {
    value.contains("\\u")
}

fn looks_like_quoted_printable_text(value: &str) -> bool {
    value.as_bytes().windows(3).any(|window| {
        window[0] == b'=' && window[1].is_ascii_hexdigit() && window[2].is_ascii_hexdigit()
    }) || value.contains("=\n")
        || value.contains("=\r\n")
}

fn looks_like_hex_text(value: &str) -> bool {
    let compact = strip_ascii_ws(value);
    compact.len() >= 4
        && compact.len().is_multiple_of(2)
        && compact.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn looks_like_binary_text(value: &str) -> bool {
    let tokens = split_code_tokens(value);
    if tokens.len() >= 2 {
        return tokens
            .iter()
            .all(|token| token.len() == 8 && token.chars().all(|ch| matches!(ch, '0' | '1')));
    }
    let compact = strip_ascii_ws(value);
    compact.len() >= 8
        && compact.len().is_multiple_of(8)
        && compact.chars().all(|ch| matches!(ch, '0' | '1'))
}

fn looks_like_octal_text(value: &str) -> bool {
    let tokens = split_code_tokens(value);
    tokens.len() >= 2
        && tokens.iter().all(|token| {
            token.len() == 3
                && token.chars().all(|ch| matches!(ch, '0'..='7'))
                && u16::from_str_radix(token, 8).is_ok_and(|byte| byte <= 0xff)
        })
}

fn looks_like_decimal_byte_text(value: &str) -> bool {
    let tokens = split_code_tokens(value);
    tokens.len() >= 2
        && tokens.iter().all(|token| {
            token.chars().all(|ch| ch.is_ascii_digit())
                && token.parse::<u16>().is_ok_and(|byte| byte <= 0xff)
        })
}

fn looks_like_ascii_code_text(value: &str) -> bool {
    let tokens = split_code_tokens(value);
    tokens.len() >= 2
        && tokens.iter().all(|token| {
            token.chars().all(|ch| ch.is_ascii_digit())
                && token
                    .parse::<u32>()
                    .is_ok_and(|codepoint| codepoint <= 0x10ffff)
        })
}

fn looks_like_rot13_text(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    ["synt{", "pgs{", "uggc", "uryyb", "cnffjbeq", "nqzva"]
        .iter()
        .any(|marker| lower.contains(marker))
}

fn looks_like_morse_text(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.len() >= 3
        && trimmed.chars().any(|ch| matches!(ch, '.' | '-'))
        && trimmed
            .chars()
            .all(|ch| matches!(ch, '.' | '-' | '/' | '|' | ' ' | '\n' | '\r' | '\t'))
}

fn split_code_tokens(value: &str) -> Vec<&str> {
    value
        .split(|ch: char| ch.is_ascii_whitespace() || matches!(ch, ',' | ';' | '|'))
        .filter(|token| !token.is_empty())
        .collect()
}

fn format_auto_decode_candidates(candidates: &[AutoCandidate]) -> String {
    if candidates.is_empty() {
        return "No confident decode candidates found.".to_string();
    }

    candidates
        .iter()
        .take(8)
        .enumerate()
        .map(|(index, candidate)| {
            format!(
                "Candidate #{number}\nSteps: {path}\nScore: {score:.3}\nResult:\n{value}",
                number = index + 1,
                path = candidate.path,
                score = candidate.score,
                value = candidate.value,
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn dummy_operation(id: &str) -> OperationSpec {
    OperationSpec {
        id: id.to_string(),
        name_zh: id.to_string(),
        name_en: id.to_string(),
        category: "auto".to_string(),
        aliases: vec![],
        input: vec!["text".to_string()],
        output: vec!["text".to_string()],
        backend: "rust".to_string(),
        safety: "safe".to_string(),
        deterministic: true,
        batchable: true,
        priority: "P1".to_string(),
        secrets: None,
    }
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
    fn base64_decodes_text() {
        let response = base64_decode(&dummy_spec(), &request("base64.decode", "ZmxhZ3t0ZXN0fQ=="))
            .expect("base64 should decode");
        assert_eq!(response.outputs[0].value, "flag{test}");
    }

    #[test]
    fn base64_offsets_show_shifted_candidates() {
        let response = base64_offsets(
            &dummy_spec(),
            &request("base64.offsets", "xZmxhZ3t0ZXN0fQ=="),
        )
        .expect("base64 offsets");
        assert!(response.outputs[0].value.contains("\"offset\": 1"));
        assert!(response.outputs[0].value.contains("flag{test}"));
    }

    #[test]
    fn cyberchef_base_codecs_round_trip() {
        for (encode, decode, expected) in [
            (
                base45_encode as fn(&OperationSpec, &OperationRequest) -> Result<OperationResponse>,
                base45_decode as fn(&OperationSpec, &OperationRequest) -> Result<OperationResponse>,
                "U.C5ECERF6$CVWE",
            ),
            (base58_encode, base58_decode, "6kkZv8vnrSZmwa"),
            (base62_encode, base62_decode, "2Pv0yKojtq1Aqb"),
            (base85_encode, base85_decode, "Ao(mgHZWh?FF="),
        ] {
            let encoded = encode(&dummy_spec(), &request("codec.encode", "flag{test}"))
                .expect("encode")
                .outputs[0]
                .value
                .clone();
            assert_eq!(encoded, expected);
            let decoded = decode(&dummy_spec(), &request("codec.decode", &encoded))
                .expect("decode")
                .outputs[0]
                .value
                .clone();
            assert_eq!(decoded, "flag{test}");
        }
    }

    #[test]
    fn numeric_byte_codecs_round_trip() {
        let binary = binary_encode(&dummy_spec(), &request("binary.encode", "Hi"))
            .expect("binary encode")
            .outputs[0]
            .value
            .clone();
        assert_eq!(binary, "01001000 01101001");
        assert_eq!(
            binary_decode(&dummy_spec(), &request("binary.decode", &binary))
                .expect("binary decode")
                .outputs[0]
                .value,
            "Hi"
        );

        let octal = octal_encode(&dummy_spec(), &request("octal.encode", "Hi"))
            .expect("octal encode")
            .outputs[0]
            .value
            .clone();
        assert_eq!(octal, "110 151");
        assert_eq!(
            octal_decode(&dummy_spec(), &request("octal.decode", &octal))
                .expect("octal decode")
                .outputs[0]
                .value,
            "Hi"
        );

        let decimal = decimal_encode(&dummy_spec(), &request("decimal.encode", "Hi"))
            .expect("decimal encode")
            .outputs[0]
            .value
            .clone();
        assert_eq!(decimal, "72 105");
        assert_eq!(
            decimal_decode(&dummy_spec(), &request("decimal.decode", &decimal))
                .expect("decimal decode")
                .outputs[0]
                .value,
            "Hi"
        );
    }

    #[test]
    fn hexdump_round_trips() {
        let encoded = hexdump_encode(&dummy_spec(), &request("hexdump.encode", "flag{test}"))
            .expect("hexdump encode")
            .outputs[0]
            .value
            .clone();
        assert!(encoded.contains("00000000"));
        assert!(encoded.contains("|flag{test}|"));
        let decoded = hexdump_decode(&dummy_spec(), &request("hexdump.decode", &encoded))
            .expect("hexdump decode")
            .outputs[0]
            .value
            .clone();
        assert_eq!(decoded, "flag{test}");
        let decoded = hexdump_decode(&dummy_spec(), &request("hexdump.decode", "666c6167"))
            .expect("plain hex should decode")
            .outputs[0]
            .value
            .clone();
        assert_eq!(decoded, "flag");
    }

    #[test]
    fn endian_swap_defaults_to_u32_chunks() {
        let response = endian_swap(&dummy_spec(), &request("endian.swap", "0011223344556677"))
            .expect("endian swap");
        assert_eq!(response.outputs[0].value, "3322110077665544");

        let response = endian_swap(
            &dummy_spec(),
            &request("endian.swap", "word=2 value=00112233"),
        )
        .expect("endian swap word");
        assert_eq!(response.outputs[0].value, "11003322");
    }

    #[test]
    fn url_round_trips() {
        let encoded = url_encode(&dummy_spec(), &request("url.encode", "a b?"))
            .expect("url should encode")
            .outputs[0]
            .value
            .clone();
        let decoded = url_decode(&dummy_spec(), &request("url.decode", &encoded))
            .expect("url should decode")
            .outputs[0]
            .value
            .clone();
        assert_eq!(decoded, "a b?");
    }

    #[test]
    fn quoted_printable_round_trips() {
        let encoded = quoted_printable_encode(
            &dummy_spec(),
            &request("quoted_printable.encode", "flag{测试}"),
        )
        .expect("quoted printable encode")
        .outputs[0]
            .value
            .clone();
        assert!(encoded.contains("=E6=B5=8B=E8=AF=95"));
        let decoded =
            quoted_printable_decode(&dummy_spec(), &request("quoted_printable.decode", &encoded))
                .expect("quoted printable decode")
                .outputs[0]
                .value
                .clone();
        assert_eq!(decoded, "flag{测试}");
    }

    #[test]
    fn auto_decode_finds_base64_flag() {
        let response = auto_decode(&dummy_spec(), &request("auto.decode", "ZmxhZ3t0ZXN0fQ=="))
            .expect("auto decode");
        let output = &response.outputs[0];
        assert_eq!(output.kind, "text");
        assert!(output.value.contains("Result:\nflag{test}"));
        assert!(output.value.contains("Steps: base64.decode"));
        assert!(!output.value.contains("\"candidates\""));
    }

    #[test]
    fn auto_decode_finds_nested_base64_hex_flag() {
        let response = auto_decode(
            &dummy_spec(),
            &request("auto.decode", "NjY2YzYxNjc3Yjc0NjU3Mzc0N2Q="),
        )
        .expect("auto decode");
        let output = &response.outputs[0].value;
        assert!(output.contains("Steps: base64.decode -> hex.decode"));
        assert!(output.contains("Result:\nflag{test}"));
    }

    #[test]
    fn auto_decode_finds_nested_url_base64_flag() {
        let response = auto_decode(
            &dummy_spec(),
            &request("auto.decode", "ZmxhZ3t0ZXN0fQ%3D%3D"),
        )
        .expect("auto decode");
        let output = &response.outputs[0].value;
        assert!(output.contains("Steps: url.decode -> base64.decode"));
        assert!(output.contains("Result:\nflag{test}"));
    }

    #[test]
    fn auto_decode_handles_unpadded_base64() {
        let response = auto_decode(&dummy_spec(), &request("auto.decode", "ZmxhZ3t0ZXN0fQ"))
            .expect("auto decode");
        let output = &response.outputs[0].value;
        assert!(output.contains("Steps: base64.decode"));
        assert!(output.contains("Result:\nflag{test}"));
    }

    #[test]
    fn auto_decode_scores_braced_ctf_payloads() {
        let response = auto_decode(&dummy_spec(), &request("auto.decode", "eHl6e2FiY19rZXl9"))
            .expect("auto decode");
        let output = &response.outputs[0].value;
        assert!(output.contains("Result:\nxyz{abc_key}"));
    }

    #[test]
    fn radix_convert_changes_base() {
        let response = radix_convert(
            &dummy_spec(),
            &request("radix.convert", "from=16 to=10 value=ff"),
        )
        .expect("radix");
        assert!(response.outputs[0].value.contains("\"converted\": \"255\""));
    }

    #[test]
    fn xor_bruteforce_finds_plaintext() {
        let ciphertext = hex::encode(
            b"flag{xor}"
                .iter()
                .map(|byte| byte ^ 0x42)
                .collect::<Vec<_>>(),
        );
        let response = xor_single_byte_bruteforce(
            &dummy_spec(),
            &request("xor.single_byte_bruteforce", &ciphertext),
        )
        .expect("xor");
        assert!(response.outputs[0].value.contains("flag{xor}"));
        assert!(response.outputs[0].value.contains("0x42"));
    }

    #[test]
    fn rot13_decodes_text() {
        let response =
            rot13_decode(&dummy_spec(), &request("rot13.decode", "synt{grfg}")).expect("rot13");
        assert_eq!(response.outputs[0].value, "flag{test}");
    }

    #[test]
    fn morse_round_trips() {
        let encoded = morse_encode(&dummy_spec(), &request("morse.encode", "SOS 123"))
            .expect("morse encode")
            .outputs[0]
            .value
            .clone();
        assert_eq!(encoded, "... --- ... / .---- ..--- ...--");

        let decoded = morse_decode(&dummy_spec(), &request("morse.decode", &encoded))
            .expect("morse decode")
            .outputs[0]
            .value
            .clone();
        assert_eq!(decoded, "SOS 123");
    }

    #[test]
    fn brainfuck_outputs_text() {
        let response = brainfuck_run(
            &dummy_spec(),
            &request("brainfuck.run", "+++++[>+++++++++++++<-]>."),
        )
        .expect("brainfuck");
        assert_eq!(response.outputs[0].value, "A");
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
