//! HTTP, JWT, and web helper implementations.

use base64::Engine;
use ctf_core::{
    CtfError, OperationOutput, OperationRequest, OperationResponse, OperationRunner, OperationSpec,
    Result,
};
use regex::Regex;
use serde::Serialize;
use sha2::{Digest, Sha256};

pub fn register_handlers(runner: &mut OperationRunner) {
    runner.register_handler("http.raw.parse", http_raw_parse);
    runner.register_handler("http.raw.to_python_requests", http_raw_to_python_requests);
    runner.register_handler("http.raw.to_python_httpx", http_raw_to_python_httpx);
    runner.register_handler("http.raw.to_curl", http_raw_to_curl);
    runner.register_handler("http.raw.to_fetch", http_raw_to_fetch);
    runner.register_handler("jwt.decode", jwt_decode);
    runner.register_handler("jwt.hs256.sign", jwt_hs256_sign);
    runner.register_handler("jwt.hs256.verify", jwt_hs256_verify);
    runner.register_handler("jwt.hs256.weak_key", jwt_hs256_weak_key);
    runner.register_handler("assets.classify", assets_classify);
}

#[derive(Debug, Clone, Serialize)]
struct HttpRequestModel {
    method: String,
    target: String,
    version: String,
    url: String,
    headers: Vec<(String, String)>,
    cookies: Vec<(String, String)>,
    body: String,
}

fn http_raw_parse(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let model = parse_http_request(&request.input_text()?)?;
    let value = serde_json::to_string_pretty(&model)
        .map_err(|error| CtfError::InvalidInput(error.to_string()))?;
    Ok(single_output("json", "request", value))
}

fn http_raw_to_python_requests(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let model = parse_http_request(&request.input_text()?)?;
    Ok(single_output(
        "code.python",
        "requests",
        python_requests(&model, "requests"),
    ))
}

fn http_raw_to_python_httpx(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let model = parse_http_request(&request.input_text()?)?;
    Ok(single_output(
        "code.python",
        "httpx",
        python_requests(&model, "httpx"),
    ))
}

fn http_raw_to_curl(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let model = parse_http_request(&request.input_text()?)?;
    Ok(single_output("code.shell", "curl", curl_command(&model)))
}

fn http_raw_to_fetch(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let model = parse_http_request(&request.input_text()?)?;
    Ok(single_output(
        "code.javascript",
        "fetch",
        fetch_code(&model),
    ))
}

fn parse_http_request(raw: &str) -> Result<HttpRequestModel> {
    let normalized = raw.replace("\r\n", "\n");
    let (head, body) = normalized
        .split_once("\n\n")
        .map_or((normalized.as_str(), ""), |(head, body)| (head, body));
    let mut lines = head.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| CtfError::InvalidInput("missing request line".to_string()))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| CtfError::InvalidInput("missing HTTP method".to_string()))?
        .to_uppercase();
    let target = parts
        .next()
        .ok_or_else(|| CtfError::InvalidInput("missing HTTP target".to_string()))?
        .to_string();
    let version = parts.next().unwrap_or("HTTP/1.1").to_string();

    let mut headers = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(CtfError::InvalidInput(format!(
                "invalid header line: {line}"
            )));
        };
        headers.push((name.trim().to_string(), value.trim().to_string()));
    }

    let host = header_value(&headers, "host")
        .ok_or_else(|| CtfError::InvalidInput("missing Host header".to_string()))?;
    let url = if target.starts_with("http://") || target.starts_with("https://") {
        target.clone()
    } else {
        format!("https://{host}{target}")
    };

    let cookies = header_value(&headers, "cookie")
        .map(parse_cookie_header)
        .unwrap_or_default();

    Ok(HttpRequestModel {
        method,
        target,
        version,
        url,
        headers,
        cookies,
        body: body.to_string(),
    })
}

fn python_requests(model: &HttpRequestModel, module: &str) -> String {
    let mut code = String::new();
    code.push_str(&format!("import {module}\n\n"));
    code.push_str(&format!("url = {:?}\n", model.url));
    code.push_str("headers = {\n");
    for (name, value) in headers_without_cookie_or_length(model) {
        code.push_str(&format!(
            "    {:?}: {:?},\n",
            name,
            redact_header(name, value)
        ));
    }
    code.push_str("}\n");
    if !model.cookies.is_empty() {
        code.push_str("cookies = {\n");
        for (name, value) in &model.cookies {
            code.push_str(&format!(
                "    {:?}: {:?},\n",
                name,
                redact_secret(name, value)
            ));
        }
        code.push_str("}\n");
    } else {
        code.push_str("cookies = {}\n");
    }
    if !model.body.is_empty() {
        code.push_str(&format!("data = {:?}\n", model.body));
    }
    code.push_str("\nresponse = ");
    code.push_str(module);
    code.push('.');
    code.push_str(&model.method.to_lowercase());
    code.push_str("(\n");
    code.push_str("    url,\n");
    code.push_str("    headers=headers,\n");
    code.push_str("    cookies=cookies,\n");
    if !model.body.is_empty() {
        code.push_str("    data=data,\n");
    }
    code.push_str("    timeout=10,\n");
    code.push_str("    allow_redirects=False,\n");
    code.push_str(")\n");
    code.push_str("print(response.status_code)\nprint(response.text)\n");
    code
}

fn curl_command(model: &HttpRequestModel) -> String {
    let mut parts = vec![
        "curl".to_string(),
        "-i".to_string(),
        "-X".to_string(),
        shell_quote(&model.method),
        shell_quote(&model.url),
    ];
    for (name, value) in &model.headers {
        if name.eq_ignore_ascii_case("content-length") {
            continue;
        }
        parts.push("-H".to_string());
        parts.push(shell_quote(&format!(
            "{}: {}",
            name,
            redact_header(name, value)
        )));
    }
    if !model.body.is_empty() {
        parts.push("--data-raw".to_string());
        parts.push(shell_quote(&model.body));
    }
    parts.join(" ")
}

fn fetch_code(model: &HttpRequestModel) -> String {
    let headers = headers_without_cookie_or_length(model)
        .into_iter()
        .map(|(name, value)| format!("    {:?}: {:?},", name, redact_header(name, value)))
        .collect::<Vec<_>>()
        .join("\n");
    let body = if model.body.is_empty() {
        String::new()
    } else {
        format!("    body: {:?},\n", model.body)
    };
    format!(
        "const response = await fetch({:?}, {{\n    method: {:?},\n    headers: {{\n{}\n    }},\n{}    redirect: \"manual\",\n}});\nconsole.log(response.status);\nconsole.log(await response.text());\n",
        model.url, model.method, headers, body
    )
}

fn jwt_decode(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let token = request.input_text()?;
    let parts = token.trim().split('.').collect::<Vec<_>>();
    if parts.len() < 2 {
        return Err(CtfError::InvalidInput(
            "JWT must contain at least header and payload".to_string(),
        ));
    }
    let header = decode_jwt_part(parts[0])?;
    let payload = decode_jwt_part(parts[1])?;
    let mut warnings = Vec::new();
    if header
        .get("alg")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|alg| alg.eq_ignore_ascii_case("none"))
    {
        warnings.push("JWT alg is none".to_string());
    }
    let value = serde_json::json!({
        "header": header,
        "payload": payload,
        "signature_present": parts.get(2).is_some_and(|part| !part.is_empty()),
    });
    Ok(OperationResponse {
        status: "ok".to_string(),
        outputs: vec![OperationOutput {
            kind: "json".to_string(),
            label: "jwt".to_string(),
            value: serde_json::to_string_pretty(&value)
                .map_err(|error| CtfError::InvalidInput(error.to_string()))?,
        }],
        warnings,
    })
}

fn jwt_hs256_sign(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let signing = parse_jwt_sign_input(&request.input_text()?)?;
    let header = signing.header.unwrap_or_else(|| {
        serde_json::json!({
            "alg": "HS256",
            "typ": "JWT",
        })
    });
    if !header
        .get("alg")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|alg| alg.eq_ignore_ascii_case("HS256"))
    {
        return Err(CtfError::InvalidInput(
            "jwt.hs256.sign requires header alg HS256".to_string(),
        ));
    }
    let token = sign_hs256(&header, &signing.payload, signing.secret.as_bytes())?;
    Ok(single_output("text", "jwt-hs256", token))
}

fn jwt_hs256_verify(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let (token, secret) = parse_token_secret_input(&request.input_text()?)?;
    let decoded = parse_jwt_token(&token)?;
    let valid = verify_hs256_signature(&token, secret.as_bytes())?;
    let mut warnings = Vec::new();
    if !decoded
        .header
        .get("alg")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|alg| alg.eq_ignore_ascii_case("HS256"))
    {
        warnings.push("JWT alg is not HS256".to_string());
    }
    Ok(OperationResponse {
        status: "ok".to_string(),
        outputs: vec![OperationOutput {
            kind: "json".to_string(),
            label: "jwt-hs256-verify".to_string(),
            value: serde_json::to_string_pretty(&serde_json::json!({
                "valid": valid,
                "header": decoded.header,
                "payload": decoded.payload,
            }))
            .map_err(|error| CtfError::InvalidInput(error.to_string()))?,
        }],
        warnings,
    })
}

fn jwt_hs256_weak_key(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let (token, candidates) = parse_token_dictionary_input(&request.input_text()?);
    let decoded = parse_jwt_token(&token)?;
    let mut warnings = Vec::new();
    if !decoded
        .header
        .get("alg")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|alg| alg.eq_ignore_ascii_case("HS256"))
    {
        warnings.push("JWT alg is not HS256".to_string());
    }

    let found = candidates
        .iter()
        .find(|secret| verify_hs256_signature(&token, secret.as_bytes()).unwrap_or(false))
        .cloned();
    Ok(OperationResponse {
        status: "ok".to_string(),
        outputs: vec![OperationOutput {
            kind: "json".to_string(),
            label: "jwt-hs256-weak-key".to_string(),
            value: serde_json::to_string_pretty(&serde_json::json!({
                "found": found.is_some(),
                "secret": found,
                "tested": candidates.len(),
            }))
            .map_err(|error| CtfError::InvalidInput(error.to_string()))?,
        }],
        warnings,
    })
}

fn assets_classify(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let text = request.input_text()?;
    let urls = captures(&text, r#"https?://[^\s"'<>]+"#)?;
    let ips = captures(
        &text,
        r#"\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)\b"#,
    )?;
    let emails = captures(
        &text,
        r#"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b"#,
    )?;
    let phones = captures(&text, r#"\b1[3-9]\d{9}\b"#)?;
    let id_cards = captures(&text, r#"\b\d{17}[\dXx]\b"#)?;
    let domains = extract_domains(&text)?;
    let c_classes = ips
        .iter()
        .filter_map(|ip| {
            let mut parts = ip.split('.');
            Some(format!(
                "{}.{}.{}.0/24",
                parts.next()?,
                parts.next()?,
                parts.next()?
            ))
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let value = serde_json::json!({
        "urls": urls,
        "domains": domains,
        "ips": ips,
        "c_classes": c_classes,
        "emails": emails,
        "phones": phones,
        "id_cards": id_cards,
        "scan_summary": {
            "possible_fscan_lines": text.lines().filter(|line| line.contains("[+]") || line.contains("open")).count(),
            "possible_vulnerability_lines": text.lines().filter(|line| line.to_ascii_lowercase().contains("poc") || line.contains("漏洞")).count(),
        }
    });

    Ok(single_output(
        "json",
        "assets",
        serde_json::to_string_pretty(&value)
            .map_err(|error| CtfError::InvalidInput(error.to_string()))?,
    ))
}

fn decode_jwt_part(part: &str) -> Result<serde_json::Value> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(part)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(part))
        .map_err(|error| CtfError::InvalidInput(format!("invalid jwt base64: {error}")))?;
    serde_json::from_slice(&bytes).map_err(|error| CtfError::InvalidInput(error.to_string()))
}

#[derive(Debug, Clone)]
struct ParsedJwt {
    header: serde_json::Value,
    payload: serde_json::Value,
    signing_input: String,
    signature: Vec<u8>,
}

#[derive(Debug, Clone)]
struct JwtSignInput {
    secret: String,
    header: Option<serde_json::Value>,
    payload: serde_json::Value,
}

fn parse_jwt_token(token: &str) -> Result<ParsedJwt> {
    let token = token.trim();
    let parts = token.split('.').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(CtfError::InvalidInput(
            "JWT must contain header.payload.signature".to_string(),
        ));
    }
    let header = decode_jwt_part(parts[0])?;
    let payload = decode_jwt_part(parts[1])?;
    let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[2])
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(parts[2]))
        .map_err(|error| CtfError::InvalidInput(format!("invalid jwt signature: {error}")))?;
    Ok(ParsedJwt {
        header,
        payload,
        signing_input: format!("{}.{}", parts[0], parts[1]),
        signature,
    })
}

fn parse_jwt_sign_input(text: &str) -> Result<JwtSignInput> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| CtfError::InvalidInput(error.to_string()))?;
    let secret = value
        .get("secret")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| CtfError::InvalidInput("missing JSON field `secret`".to_string()))?
        .to_string();
    let payload = value
        .get("payload")
        .cloned()
        .ok_or_else(|| CtfError::InvalidInput("missing JSON field `payload`".to_string()))?;
    let header = value.get("header").cloned();
    Ok(JwtSignInput {
        secret,
        header,
        payload,
    })
}

fn parse_token_secret_input(text: &str) -> Result<(String, String)> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
        let token = value
            .get("token")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CtfError::InvalidInput("missing JSON field `token`".to_string()))?
            .to_string();
        let secret = value
            .get("secret")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CtfError::InvalidInput("missing JSON field `secret`".to_string()))?
            .to_string();
        return Ok((token, secret));
    }

    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let token = lines
        .next()
        .ok_or_else(|| CtfError::InvalidInput("missing JWT token".to_string()))?
        .trim_start_matches("token=")
        .to_string();
    let secret = lines
        .next()
        .ok_or_else(|| CtfError::InvalidInput("missing HS256 secret".to_string()))?
        .trim_start_matches("secret=")
        .to_string();
    Ok((token, secret))
}

fn parse_token_dictionary_input(text: &str) -> (String, Vec<String>) {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let token = lines
        .next()
        .unwrap_or_default()
        .trim_start_matches("token=")
        .to_string();
    let mut candidates = lines
        .map(|line| line.trim_start_matches("secret=").to_string())
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        candidates = [
            "secret", "password", "123456", "admin", "jwt", "key", "test", "flag", "ctf",
            "changeme",
        ]
        .iter()
        .map(|item| item.to_string())
        .collect();
    }
    (token, candidates)
}

fn sign_hs256(
    header: &serde_json::Value,
    payload: &serde_json::Value,
    secret: &[u8],
) -> Result<String> {
    let header_json =
        serde_json::to_vec(header).map_err(|error| CtfError::InvalidInput(error.to_string()))?;
    let payload_json =
        serde_json::to_vec(payload).map_err(|error| CtfError::InvalidInput(error.to_string()))?;
    let header_part = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header_json);
    let payload_part = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_json);
    let signing_input = format!("{header_part}.{payload_part}");
    let signature = hmac_sha256(secret, signing_input.as_bytes());
    let signature_part = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature);
    Ok(format!("{signing_input}.{signature_part}"))
}

fn verify_hs256_signature(token: &str, secret: &[u8]) -> Result<bool> {
    let parsed = parse_jwt_token(token)?;
    let expected = hmac_sha256(secret, parsed.signing_input.as_bytes());
    Ok(constant_time_eq(&expected, &parsed.signature))
}

fn hmac_sha256(secret: &[u8], message: &[u8]) -> Vec<u8> {
    const BLOCK_SIZE: usize = 64;
    let mut key = if secret.len() > BLOCK_SIZE {
        Sha256::digest(secret).to_vec()
    } else {
        secret.to_vec()
    };
    key.resize(BLOCK_SIZE, 0);

    let mut ipad = [0x36u8; BLOCK_SIZE];
    let mut opad = [0x5cu8; BLOCK_SIZE];
    for (index, key_byte) in key.iter().enumerate() {
        ipad[index] ^= key_byte;
        opad[index] ^= key_byte;
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_digest);
    outer.finalize().to_vec()
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |acc, (left, right)| acc | (left ^ right))
        == 0
}

fn captures(text: &str, pattern: &str) -> Result<Vec<String>> {
    let regex = Regex::new(pattern).map_err(|error| CtfError::InvalidInput(error.to_string()))?;
    Ok(regex
        .find_iter(text)
        .map(|item| item.as_str().trim_end_matches(['.', ',', ';']).to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect())
}

fn extract_domains(text: &str) -> Result<Vec<String>> {
    let mut domains = captures(text, r#"\b(?:[A-Za-z0-9-]+\.)+[A-Za-z]{2,}\b"#)?;
    domains.retain(|domain| !domain.contains('@'));
    Ok(domains)
}

fn header_value(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(header, _)| header.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.clone())
}

fn headers_without_cookie_or_length(model: &HttpRequestModel) -> Vec<(&String, &String)> {
    model
        .headers
        .iter()
        .filter(|(name, _)| {
            !name.eq_ignore_ascii_case("cookie") && !name.eq_ignore_ascii_case("content-length")
        })
        .map(|(name, value)| (name, value))
        .collect()
}

fn parse_cookie_header(value: String) -> Vec<(String, String)> {
    value
        .split(';')
        .filter_map(|part| {
            let (name, value) = part.trim().split_once('=')?;
            Some((name.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

fn redact_header(name: &str, value: &str) -> String {
    if is_secret_name(name) {
        "REPLACE_ME".to_string()
    } else {
        value.to_string()
    }
}

fn redact_secret(name: &str, value: &str) -> String {
    if is_secret_name(name) || !value.is_empty() {
        "REPLACE_ME".to_string()
    } else {
        value.to_string()
    }
}

fn is_secret_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.contains("cookie")
        || name.contains("authorization")
        || name.contains("token")
        || name.contains("api-key")
        || name.contains("apikey")
        || name == "session"
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
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
    fn parses_raw_request() {
        let raw = "POST /login HTTP/1.1\r\nHost: example.com\r\nCookie: session=abc\r\nContent-Length: 3\r\n\r\na=1";
        let model = parse_http_request(raw).expect("request should parse");
        assert_eq!(model.method, "POST");
        assert_eq!(model.url, "https://example.com/login");
        assert_eq!(model.cookies[0].0, "session");
    }

    #[test]
    fn codegen_redacts_cookie() {
        let raw = "GET / HTTP/1.1\nHost: example.com\nCookie: session=abc\n\n";
        let response = http_raw_to_python_requests(
            &dummy_spec(),
            &request("http.raw.to_python_requests", raw),
        )
        .expect("codegen should run");
        assert!(response.outputs[0].value.contains("REPLACE_ME"));
        assert!(!response.outputs[0].value.contains("abc"));
    }

    #[test]
    fn jwt_decodes_payload() {
        let token = "eyJhbGciOiJub25lIn0.eyJzdWIiOiIxMjMifQ.";
        let response = jwt_decode(&dummy_spec(), &request("jwt.decode", token)).expect("jwt");
        assert!(response.outputs[0].value.contains("\"sub\": \"123\""));
        assert_eq!(response.warnings[0], "JWT alg is none");
    }

    #[test]
    fn assets_classify_groups_targets() {
        let response = assets_classify(
            &dummy_spec(),
            &request(
                "assets.classify",
                "https://a.example.com/login 192.168.1.9 admin@example.com 13800138000",
            ),
        )
        .expect("assets");
        let value = &response.outputs[0].value;
        assert!(value.contains("a.example.com"));
        assert!(value.contains("192.168.1.0/24"));
        assert!(value.contains("admin@example.com"));
    }

    #[test]
    fn jwt_hs256_sign_verify_and_weak_key() {
        let sign_input = r#"{"secret":"secret","payload":{"sub":"123"}}"#;
        let token = jwt_hs256_sign(&dummy_spec(), &request("jwt.hs256.sign", sign_input))
            .expect("sign")
            .outputs[0]
            .value
            .clone();

        let verify_input = format!("{token}\nsecret");
        let verified = jwt_hs256_verify(&dummy_spec(), &request("jwt.hs256.verify", &verify_input))
            .expect("verify");
        assert!(verified.outputs[0].value.contains("\"valid\": true"));

        let weak = jwt_hs256_weak_key(
            &dummy_spec(),
            &request("jwt.hs256.weak_key", &format!("{token}\nadmin\nsecret")),
        )
        .expect("weak key");
        assert!(weak.outputs[0].value.contains("\"secret\": \"secret\""));
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
