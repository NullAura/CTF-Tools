//! File, image, stego, and forensics helpers.

use ctf_core::{
    CtfError, OperationOutput, OperationRequest, OperationResponse, OperationRunner, OperationSpec,
    Result,
};
use regex::Regex;

pub fn register_handlers(runner: &mut OperationRunner) {
    runner.register_handler("file.hex_view", file_hex_view);
    runner.register_handler("file.entropy", file_entropy);
    runner.register_handler("pcap.http.extract", pcap_http_extract);
}

fn file_hex_view(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let bytes = request.input_bytes()?;
    Ok(single_output("text", "hex-view", hex_view(&bytes, 512)))
}

fn file_entropy(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let bytes = request.input_bytes()?;
    let entropy = shannon_entropy(&bytes);
    let value = serde_json::json!({
        "bytes": bytes.len(),
        "entropy": entropy,
    });
    Ok(single_output("json", "entropy", value.to_string()))
}

fn pcap_http_extract(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let bytes = request.input_bytes()?;
    let text = String::from_utf8_lossy(&bytes);
    let request_re =
        Regex::new(r"(?m)\b(GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS) [^\r\n]+ HTTP/1\.[01]")
            .map_err(|error| CtfError::InvalidInput(error.to_string()))?;
    let response_re = Regex::new(r"(?m)\bHTTP/1\.[01] \d{3} [^\r\n]+")
        .map_err(|error| CtfError::InvalidInput(error.to_string()))?;
    let requests = request_re
        .find_iter(&text)
        .map(|item| item.as_str().to_string())
        .collect::<Vec<_>>();
    let responses = response_re
        .find_iter(&text)
        .map(|item| item.as_str().to_string())
        .collect::<Vec<_>>();
    let value = serde_json::json!({
        "http_requests": requests,
        "http_responses": responses,
        "note": "lightweight string extraction; full stream reassembly is handled by optional tshark/scapy plugins"
    });
    Ok(single_output(
        "json",
        "pcap-http",
        serde_json::to_string_pretty(&value)
            .map_err(|error| CtfError::InvalidInput(error.to_string()))?,
    ))
}

fn hex_view(bytes: &[u8], max: usize) -> String {
    let mut out = String::new();
    for (row, chunk) in bytes
        .iter()
        .take(max)
        .collect::<Vec<_>>()
        .chunks(16)
        .enumerate()
    {
        let offset = row * 16;
        let hex = chunk
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<Vec<_>>()
            .join(" ");
        let ascii = chunk
            .iter()
            .map(|byte| {
                let byte = **byte;
                if byte.is_ascii_graphic() || byte == b' ' {
                    byte as char
                } else {
                    '.'
                }
            })
            .collect::<String>();
        out.push_str(&format!("{offset:08x}  {hex:<47}  {ascii}\n"));
    }
    if bytes.len() > max {
        out.push_str(&format!("... truncated, total {} bytes\n", bytes.len()));
    }
    out
}

fn shannon_entropy(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }
    let mut counts = [0usize; 256];
    for byte in bytes {
        counts[*byte as usize] += 1;
    }
    let len = bytes.len() as f64;
    counts
        .iter()
        .filter(|count| **count > 0)
        .map(|count| {
            let p = *count as f64 / len;
            -p * p.log2()
        })
        .sum()
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
    fn hex_view_displays_ascii() {
        let response =
            file_hex_view(&dummy_spec(), &request("file.hex_view", "flag")).expect("hex view");
        assert!(response.outputs[0].value.contains("66 6c 61 67"));
        assert!(response.outputs[0].value.contains("flag"));
    }

    #[test]
    fn pcap_extract_finds_http_line() {
        let response = pcap_http_extract(
            &dummy_spec(),
            &request(
                "pcap.http.extract",
                "noise GET / HTTP/1.1\r\nHost: example.com\r\n",
            ),
        )
        .expect("pcap extract");
        assert!(response.outputs[0].value.contains("GET / HTTP/1.1"));
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
