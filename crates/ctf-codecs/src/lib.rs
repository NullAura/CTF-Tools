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
