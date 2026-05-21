//! Text codec implementations.

use base64::Engine;
use ctf_core::{
    CtfError, OperationOutput, OperationRequest, OperationResponse, OperationRunner, OperationSpec,
    Result,
};
use data_encoding::{BASE32, BASE32_NOPAD};

pub fn register_handlers(runner: &mut OperationRunner) {
    runner.register_handler("base64.decode", base64_decode);
    runner.register_handler("base64.encode", base64_encode);
    runner.register_handler("base32.decode", base32_decode);
    runner.register_handler("base32.encode", base32_encode);
    runner.register_handler("url.decode", url_decode);
    runner.register_handler("url.encode", url_encode);
    runner.register_handler("html.decode", html_decode);
    runner.register_handler("html.encode", html_encode);
    runner.register_handler("unicode.decode", unicode_decode);
    runner.register_handler("unicode.encode", unicode_encode);
    runner.register_handler("ascii.decode", ascii_decode);
    runner.register_handler("ascii.encode", ascii_encode);
    runner.register_handler("hex.decode", hex_decode);
    runner.register_handler("hex.encode", hex_encode);
    runner.register_handler("radix.convert", radix_convert);
    runner.register_handler("xor.single_byte_bruteforce", xor_single_byte_bruteforce);
    runner.register_handler("auto.decode", auto_decode);
}

fn base64_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let text = strip_ascii_ws(&request.input_text()?);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(text.as_bytes())
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(text.as_bytes()))
        .map_err(|error| CtfError::InvalidInput(format!("invalid base64: {error}")))?;
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

fn auto_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let input = request.input_text()?;
    let mut candidates = Vec::new();

    if let Ok(response) = base64_decode(&dummy_operation("base64.decode"), request) {
        push_candidate(&mut candidates, "base64.decode", &response.outputs[0].value);
    }

    if let Ok(response) = hex_decode(&dummy_operation("hex.decode"), request) {
        push_candidate(&mut candidates, "hex.decode", &response.outputs[0].value);
    }

    if input.contains('%')
        && let Ok(response) = url_decode(&dummy_operation("url.decode"), request)
    {
        push_candidate(&mut candidates, "url.decode", &response.outputs[0].value);
    }

    if input.contains("\\u")
        && let Ok(response) = unicode_decode(&dummy_operation("unicode.decode"), request)
    {
        push_candidate(
            &mut candidates,
            "unicode.decode",
            &response.outputs[0].value,
        );
    }

    if input
        .chars()
        .all(|ch| ch.is_ascii_digit() || ch.is_ascii_whitespace() || ch == ',' || ch == ';')
        && let Ok(response) = ascii_decode(&dummy_operation("ascii.decode"), request)
    {
        push_candidate(&mut candidates, "ascii.decode", &response.outputs[0].value);
    }

    candidates.sort_by(|a: &AutoCandidate, b| b.score.total_cmp(&a.score));
    let value = serde_json_like_candidates(&candidates);
    Ok(single_output("json", "auto", value))
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

struct AutoCandidate {
    path: String,
    score: f32,
    value: String,
}

fn push_candidate(candidates: &mut Vec<AutoCandidate>, path: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    candidates.push(AutoCandidate {
        path: path.to_string(),
        score: score_text(value),
        value: value.to_string(),
    });
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
        score += 1.0;
    }
    score
}

fn serde_json_like_candidates(candidates: &[AutoCandidate]) -> String {
    let items = candidates
        .iter()
        .take(8)
        .map(|candidate| {
            format!(
                "{{\n    \"path\": {:?},\n    \"score\": {:.3},\n    \"value\": {:?}\n  }}",
                candidate.path, candidate.score, candidate.value
            )
        })
        .collect::<Vec<_>>()
        .join(",\n  ");
    format!("{{\n  \"candidates\": [\n  {items}\n  ]\n}}")
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
    fn auto_decode_finds_base64_flag() {
        let response = auto_decode(&dummy_spec(), &request("auto.decode", "ZmxhZ3t0ZXN0fQ=="))
            .expect("auto decode");
        assert!(response.outputs[0].value.contains("flag{test}"));
        assert!(response.outputs[0].value.contains("base64.decode"));
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
