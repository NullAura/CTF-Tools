//! Crypto helpers.

use base64::Engine;
use ctf_core::{
    CtfError, OperationOutput, OperationRequest, OperationResponse, OperationRunner, OperationSpec,
    Result,
};
use md4::Md4;
use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use sha3::Sha3_256;
use sm3::Sm3;

pub fn register_handlers(runner: &mut OperationRunner) {
    runner.register_handler("hash.md5", hash_md5);
    runner.register_handler("hash.sha1", hash_sha1);
    runner.register_handler("hash.sha256", hash_sha256);
    runner.register_handler("hash.sha512", hash_sha512);
    runner.register_handler("hash.sha3_256", hash_sha3_256);
    runner.register_handler("hash.ntlm", hash_ntlm);
    runner.register_handler("hash.sm3", hash_sm3);
    runner.register_handler("rc4.apply", rc4_apply);
    runner.register_handler("xor.repeating_key", xor_repeating_key);
    runner.register_handler("vigenere.encode", vigenere_encode);
    runner.register_handler("vigenere.decode", vigenere_decode);
    runner.register_handler("atbash.decode", atbash_decode);
    runner.register_handler("rot47.decode", rot47_decode);
}

fn hash_md5(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    hash::<Md5>("md5", &request.input_bytes()?)
}

fn hash_sha1(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    hash::<Sha1>("sha1", &request.input_bytes()?)
}

fn hash_sha256(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    hash::<Sha256>("sha256", &request.input_bytes()?)
}

fn hash_sha512(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    hash::<Sha512>("sha512", &request.input_bytes()?)
}

fn hash_sha3_256(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    hash::<Sha3_256>("sha3-256", &request.input_bytes()?)
}

fn hash_sm3(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    hash::<Sm3>("sm3", &request.input_bytes()?)
}

fn hash_ntlm(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let mut bytes = Vec::new();
    for code in request.input_text()?.encode_utf16() {
        bytes.extend_from_slice(&code.to_le_bytes());
    }
    hash::<Md4>("ntlm", &bytes)
}

fn rc4_apply(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let input = parse_keyed_input(&request.input_text()?)?;
    let key = decode_configured_bytes(&input.key, input.key_encoding.as_deref(), false)?;
    if key.is_empty() {
        return Err(CtfError::InvalidInput("key is empty".to_string()));
    }
    let data = decode_configured_bytes(&input.data, input.input_encoding.as_deref(), true)?;
    let result = rc4_crypt(&key, &data);
    Ok(single_output(
        "text",
        "rc4",
        format_configured_bytes(&result, input.output_encoding.as_deref())?,
    ))
}

fn xor_repeating_key(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let input = parse_keyed_input(&request.input_text()?)?;
    let key = decode_configured_bytes(&input.key, input.key_encoding.as_deref(), false)?;
    if key.is_empty() {
        return Err(CtfError::InvalidInput("key is empty".to_string()));
    }
    let data = decode_configured_bytes(&input.data, input.input_encoding.as_deref(), true)?;
    let result = data
        .iter()
        .zip(key.iter().cycle())
        .map(|(byte, key)| byte ^ key)
        .collect::<Vec<_>>();
    Ok(single_output(
        "text",
        "xor",
        format_configured_bytes(&result, input.output_encoding.as_deref())?,
    ))
}

fn vigenere_encode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let input = parse_keyed_input(&request.input_text()?)?;
    Ok(single_output(
        "text",
        "vigenere",
        vigenere_transform(&input.data, &input.key, true)?,
    ))
}

fn vigenere_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let input = parse_keyed_input(&request.input_text()?)?;
    Ok(single_output(
        "text",
        "vigenere",
        vigenere_transform(&input.data, &input.key, false)?,
    ))
}

fn atbash_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    Ok(single_output(
        "text",
        "atbash",
        request.input_text()?.chars().map(atbash_char).collect(),
    ))
}

fn rot47_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    Ok(single_output(
        "text",
        "rot47",
        request.input_text()?.chars().map(rot47_char).collect(),
    ))
}

fn hash<D>(label: &str, bytes: &[u8]) -> Result<OperationResponse>
where
    D: Digest + Default,
{
    let digest = D::digest(bytes);
    Ok(OperationResponse {
        status: "ok".to_string(),
        outputs: vec![OperationOutput {
            kind: "text".to_string(),
            label: label.to_string(),
            value: hex::encode(digest),
        }],
        warnings: vec![],
    })
}

#[derive(Debug, Clone, Default)]
struct KeyedInput {
    key: String,
    data: String,
    input_encoding: Option<String>,
    output_encoding: Option<String>,
    key_encoding: Option<String>,
}

fn parse_keyed_input(text: &str) -> Result<KeyedInput> {
    let normalized = text.replace("\r\n", "\n");
    let mut parsed = KeyedInput::default();
    let mut body = Vec::new();
    let mut saw_settings = false;
    let mut force_body = false;

    for line in normalized.lines() {
        if !force_body && line.trim().is_empty() {
            force_body = true;
            continue;
        }
        if !force_body && let Some((name, value)) = line.split_once('=') {
            let name = name.trim().to_ascii_lowercase().replace('-', "_");
            let value = value.trim().to_string();
            match name.as_str() {
                "key" => {
                    parsed.key = value;
                    saw_settings = true;
                    continue;
                }
                "data" | "text" | "ciphertext" | "plaintext" => {
                    parsed.data = value;
                    saw_settings = true;
                    continue;
                }
                "input" | "input_encoding" | "data_encoding" => {
                    parsed.input_encoding = Some(value);
                    saw_settings = true;
                    continue;
                }
                "output" | "output_encoding" => {
                    parsed.output_encoding = Some(value);
                    saw_settings = true;
                    continue;
                }
                "key_encoding" => {
                    parsed.key_encoding = Some(value);
                    saw_settings = true;
                    continue;
                }
                _ => {}
            }
        }
        body.push(line);
    }

    if saw_settings {
        if parsed.key.is_empty() {
            return Err(CtfError::InvalidInput(
                "expected key=... before keyed crypto input".to_string(),
            ));
        }
        if parsed.data.is_empty() {
            parsed.data = body.join("\n");
        }
    } else {
        let mut lines = normalized.lines();
        parsed.key = lines.next().unwrap_or_default().trim().to_string();
        parsed.data = lines.collect::<Vec<_>>().join("\n");
    }

    if parsed.key.is_empty() {
        return Err(CtfError::InvalidInput(
            "expected keyed input, for example: key=secret\\n\\nmessage".to_string(),
        ));
    }
    if parsed.data.is_empty() {
        return Err(CtfError::InvalidInput(
            "expected data after key, for example: key=secret\\n\\nmessage".to_string(),
        ));
    }
    Ok(parsed)
}

fn decode_configured_bytes(value: &str, encoding: Option<&str>, auto_hex: bool) -> Result<Vec<u8>> {
    match encoding
        .map(|encoding| encoding.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("text" | "raw" | "utf8" | "utf-8") => Ok(value.as_bytes().to_vec()),
        Some("hex") => {
            let cleaned = strip_hex_separators(value);
            hex::decode(&cleaned)
                .map_err(|error| CtfError::InvalidInput(format!("invalid hex input: {error}")))
        }
        Some("base64" | "b64") => base64::engine::general_purpose::STANDARD
            .decode(strip_ascii_ws(value))
            .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(strip_ascii_ws(value)))
            .map_err(|error| CtfError::InvalidInput(format!("invalid base64 input: {error}"))),
        Some(other) => Err(CtfError::InvalidInput(format!(
            "unsupported encoding `{other}`; use text, hex, or base64"
        ))),
        None if auto_hex && looks_like_hex(value) => hex::decode(strip_hex_separators(value))
            .map_err(|error| CtfError::InvalidInput(format!("invalid hex input: {error}"))),
        None => Ok(value.as_bytes().to_vec()),
    }
}

fn format_configured_bytes(bytes: &[u8], encoding: Option<&str>) -> Result<String> {
    match encoding
        .map(|encoding| encoding.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("text" | "raw" | "utf8" | "utf-8") => Ok(String::from_utf8_lossy(bytes).into_owned()),
        Some("hex") => Ok(hex::encode(bytes)),
        Some("base64" | "b64") => Ok(base64::engine::general_purpose::STANDARD.encode(bytes)),
        Some(other) => Err(CtfError::InvalidInput(format!(
            "unsupported output encoding `{other}`; use text, hex, or base64"
        ))),
        None if bytes_are_display_text(bytes) => Ok(String::from_utf8_lossy(bytes).into_owned()),
        None => Ok(hex::encode(bytes)),
    }
}

fn strip_ascii_ws(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect()
}

fn strip_hex_separators(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("0x")
        .replace([' ', '\n', '\r', '\t', ':', '-'], "")
}

fn looks_like_hex(value: &str) -> bool {
    let cleaned = strip_hex_separators(value);
    cleaned.len() >= 2
        && cleaned.len().is_multiple_of(2)
        && cleaned.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn bytes_are_display_text(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .map(|text| {
            text.chars()
                .all(|ch| !ch.is_control() || matches!(ch, '\n' | '\r' | '\t'))
        })
        .unwrap_or(false)
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

fn rc4_crypt(key: &[u8], input: &[u8]) -> Vec<u8> {
    let mut s = [0u8; 256];
    for (index, value) in s.iter_mut().enumerate() {
        *value = index as u8;
    }

    let mut j = 0usize;
    for i in 0..256 {
        j = (j + s[i] as usize + key[i % key.len()] as usize) & 0xff;
        s.swap(i, j);
    }

    let mut i = 0usize;
    let mut j = 0usize;
    input
        .iter()
        .map(|byte| {
            i = (i + 1) & 0xff;
            j = (j + s[i] as usize) & 0xff;
            s.swap(i, j);
            let k = s[(s[i] as usize + s[j] as usize) & 0xff];
            byte ^ k
        })
        .collect()
}

fn vigenere_transform(text: &str, key: &str, encode: bool) -> Result<String> {
    let key_shifts = key
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.to_ascii_lowercase() as u8 - b'a')
        .collect::<Vec<_>>();
    if key_shifts.is_empty() {
        return Err(CtfError::InvalidInput(
            "Vigenere key must contain at least one ASCII letter".to_string(),
        ));
    }

    let mut key_index = 0usize;
    Ok(text
        .chars()
        .map(|ch| {
            if !ch.is_ascii_alphabetic() {
                return ch;
            }
            let shift = key_shifts[key_index % key_shifts.len()];
            key_index += 1;
            let shift = if encode { shift } else { (26 - shift) % 26 };
            shift_alpha(ch, shift)
        })
        .collect())
}

fn shift_alpha(ch: char, shift: u8) -> char {
    let base = if ch.is_ascii_uppercase() { b'A' } else { b'a' };
    (((ch as u8 - base + shift) % 26) + base) as char
}

fn atbash_char(ch: char) -> char {
    match ch {
        'a'..='z' => (b'z' - (ch as u8 - b'a')) as char,
        'A'..='Z' => (b'Z' - (ch as u8 - b'A')) as char,
        _ => ch,
    }
}

fn rot47_char(ch: char) -> char {
    let code = ch as u32;
    if (33..=126).contains(&code) {
        char::from_u32(33 + ((code - 33 + 47) % 94)).unwrap_or(ch)
    } else {
        ch
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
    fn md5_hashes_text() {
        let response = hash_md5(&dummy_spec(), &request("hash.md5", "abc")).expect("md5");
        assert_eq!(
            response.outputs[0].value,
            "900150983cd24fb0d6963f7d28e17f72"
        );
    }

    #[test]
    fn ntlm_hashes_password() {
        let response = hash_ntlm(&dummy_spec(), &request("hash.ntlm", "password")).expect("ntlm");
        assert_eq!(
            response.outputs[0].value,
            "8846f7eaee8fb117ad06bdd830b7586c"
        );
    }

    #[test]
    fn rc4_applies_known_vector() {
        let encrypted = rc4_apply(
            &dummy_spec(),
            &request("rc4.apply", "key=Key\noutput=hex\n\nPlaintext"),
        )
        .expect("rc4 encrypt");
        assert_eq!(encrypted.outputs[0].value, "bbf316e8d940af0ad3");

        let decrypted = rc4_apply(
            &dummy_spec(),
            &request(
                "rc4.apply",
                "key=Key\ninput=hex\noutput=text\n\nbbf316e8d940af0ad3",
            ),
        )
        .expect("rc4 decrypt");
        assert_eq!(decrypted.outputs[0].value, "Plaintext");
    }

    #[test]
    fn repeating_xor_round_trips() {
        let encrypted = xor_repeating_key(
            &dummy_spec(),
            &request("xor.repeating_key", "key=ICE\noutput=hex\n\nhello"),
        )
        .expect("xor encrypt");
        assert_eq!(encrypted.outputs[0].value, "212629252c");

        let decrypted = xor_repeating_key(
            &dummy_spec(),
            &request("xor.repeating_key", "key=ICE\ninput=hex\n\n212629252c"),
        )
        .expect("xor decrypt");
        assert_eq!(decrypted.outputs[0].value, "hello");
    }

    #[test]
    fn vigenere_round_trips() {
        let encoded = vigenere_encode(
            &dummy_spec(),
            &request("vigenere.encode", "key=LEMON\n\nATTACKATDAWN"),
        )
        .expect("vigenere encode");
        assert_eq!(encoded.outputs[0].value, "LXFOPVEFRNHR");

        let decoded = vigenere_decode(
            &dummy_spec(),
            &request("vigenere.decode", "key=LEMON\n\nLXFOPVEFRNHR"),
        )
        .expect("vigenere decode");
        assert_eq!(decoded.outputs[0].value, "ATTACKATDAWN");
    }

    #[test]
    fn atbash_and_rot47_decode_text() {
        let atbash =
            atbash_decode(&dummy_spec(), &request("atbash.decode", "Zgyzhs")).expect("atbash");
        assert_eq!(atbash.outputs[0].value, "Atbash");

        let rot47 = rot47_decode(&dummy_spec(), &request("rot47.decode", "w6==@")).expect("rot47");
        assert_eq!(rot47.outputs[0].value, "Hello");
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
