//! HTTP, JWT, and web helper implementations.

use base64::Engine;
use ctf_core::{
    CtfError, OperationOutput, OperationRequest, OperationResponse, OperationRunner, OperationSpec,
    Result,
};
use serde::Serialize;

pub fn register_handlers(runner: &mut OperationRunner) {
    runner.register_handler("http.raw.parse", http_raw_parse);
    runner.register_handler("http.raw.to_python_requests", http_raw_to_python_requests);
    runner.register_handler("http.raw.to_python_httpx", http_raw_to_python_httpx);
    runner.register_handler("http.raw.to_curl", http_raw_to_curl);
    runner.register_handler("http.raw.to_fetch", http_raw_to_fetch);
    runner.register_handler("jwt.decode", jwt_decode);
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

fn decode_jwt_part(part: &str) -> Result<serde_json::Value> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(part)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(part))
        .map_err(|error| CtfError::InvalidInput(format!("invalid jwt base64: {error}")))?;
    serde_json::from_slice(&bytes).map_err(|error| CtfError::InvalidInput(error.to_string()))
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
