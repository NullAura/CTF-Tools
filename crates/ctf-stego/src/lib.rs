//! File, image, stego, and forensics helpers.

use base64::{Engine as _, engine::general_purpose};
use ctf_core::{
    CtfError, OperationOutput, OperationRequest, OperationResponse, OperationRunner, OperationSpec,
    Result,
};
use regex::Regex;

pub fn register_handlers(runner: &mut OperationRunner) {
    runner.register_handler("file.hex_view", file_hex_view);
    runner.register_handler("file.entropy", file_entropy);
    runner.register_handler("file.entropy_map", file_entropy_map);
    runner.register_handler("image.base64.data_uri", image_base64_data_uri);
    runner.register_handler("image.gif.frames", image_gif_frames);
    runner.register_handler("pcap.http.extract", pcap_http_extract);
    runner.register_handler("pcap.dns.extract", pcap_dns_extract);
    runner.register_handler("pcap.icmp.extract", pcap_icmp_extract);
    runner.register_handler("pcap.tcp.summary", pcap_tcp_summary);
    runner.register_handler("usb.hid.keys", usb_hid_keys);
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
    json_output("entropy", value)
}

fn file_entropy_map(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let bytes = request.input_bytes()?;
    let chunk_size = 256usize;
    let chunks = bytes
        .chunks(chunk_size)
        .enumerate()
        .map(|(index, chunk)| {
            serde_json::json!({
                "offset": index * chunk_size,
                "length": chunk.len(),
                "entropy": shannon_entropy(chunk),
            })
        })
        .collect::<Vec<_>>();
    json_output(
        "entropy-map",
        serde_json::json!({
            "bytes": bytes.len(),
            "chunk_size": chunk_size,
            "chunks": chunks,
        }),
    )
}

fn image_base64_data_uri(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let bytes = request.input_bytes()?;
    let mime = image_mime(&bytes);
    Ok(single_output(
        "text",
        "data-uri",
        format!(
            "data:{mime};base64,{}",
            general_purpose::STANDARD.encode(bytes)
        ),
    ))
}

fn image_gif_frames(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let bytes = request.input_bytes()?;
    let frames = parse_gif_frames(&bytes)?;
    json_output(
        "gif-frames",
        serde_json::json!({
            "frames": frames,
        }),
    )
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

fn pcap_dns_extract(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let bytes = request.input_bytes()?;
    let packets = parse_pcap_packets(&bytes)?;
    let mut queries = Vec::new();
    for packet in packets {
        if let Some(ipv4) = parse_ipv4_payload(&packet.data, packet.linktype)
            && ipv4.protocol == 17
            && let Some(udp) = parse_udp_segment(ipv4.payload)
            && (udp.src_port == 53 || udp.dst_port == 53)
        {
            for query in parse_dns_queries(udp.payload) {
                queries.push(serde_json::json!({
                    "src": ipv4.src,
                    "dst": ipv4.dst,
                    "src_port": udp.src_port,
                    "dst_port": udp.dst_port,
                    "query": query,
                }));
            }
        }
    }
    json_output("pcap-dns", serde_json::json!({ "queries": queries }))
}

fn pcap_icmp_extract(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let bytes = request.input_bytes()?;
    let packets = parse_pcap_packets(&bytes)?;
    let mut messages = Vec::new();
    for packet in packets {
        if let Some(ipv4) = parse_ipv4_payload(&packet.data, packet.linktype)
            && ipv4.protocol == 1
            && ipv4.payload.len() >= 2
        {
            messages.push(serde_json::json!({
                "src": ipv4.src,
                "dst": ipv4.dst,
                "type": ipv4.payload[0],
                "code": ipv4.payload[1],
                "payload_bytes": ipv4.payload.len().saturating_sub(8),
            }));
        }
    }
    json_output("pcap-icmp", serde_json::json!({ "messages": messages }))
}

fn pcap_tcp_summary(
    _spec: &OperationSpec,
    request: &OperationRequest,
) -> Result<OperationResponse> {
    let bytes = request.input_bytes()?;
    let packets = parse_pcap_packets(&bytes)?;
    let mut flows: Vec<TcpFlowSummary> = Vec::new();
    for packet in packets {
        if let Some(ipv4) = parse_ipv4_payload(&packet.data, packet.linktype)
            && ipv4.protocol == 6
            && let Some(tcp) = parse_tcp_segment(ipv4.payload)
        {
            let key = format!(
                "{}:{} -> {}:{}",
                ipv4.src, tcp.src_port, ipv4.dst, tcp.dst_port
            );
            if let Some(flow) = flows.iter_mut().find(|flow| flow.key == key) {
                flow.packets += 1;
                flow.payload_bytes += tcp.payload.len();
            } else {
                flows.push(TcpFlowSummary {
                    key,
                    src: ipv4.src,
                    dst: ipv4.dst,
                    src_port: tcp.src_port,
                    dst_port: tcp.dst_port,
                    packets: 1,
                    payload_bytes: tcp.payload.len(),
                });
            }
        }
    }
    let flows = flows
        .into_iter()
        .map(|flow| {
            serde_json::json!({
                "src": flow.src,
                "dst": flow.dst,
                "src_port": flow.src_port,
                "dst_port": flow.dst_port,
                "packets": flow.packets,
                "payload_bytes": flow.payload_bytes,
            })
        })
        .collect::<Vec<_>>();
    json_output("pcap-tcp", serde_json::json!({ "flows": flows }))
}

fn usb_hid_keys(_spec: &OperationSpec, request: &OperationRequest) -> Result<OperationResponse> {
    let bytes = if request.input.kind == "text" {
        parse_hex_or_raw(&request.input.value)
    } else {
        request.input_bytes()?
    };
    let keys = decode_usb_hid_reports(&bytes);
    json_output(
        "usb-hid-keys",
        serde_json::json!({
            "text": keys.iter().map(|key| key.text.clone()).collect::<String>(),
            "keys": keys.into_iter().map(|key| key.name).collect::<Vec<_>>(),
        }),
    )
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

fn image_mime(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        "image/jpeg"
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        "image/gif"
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP".as_slice()) {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

fn parse_gif_frames(bytes: &[u8]) -> Result<Vec<serde_json::Value>> {
    if !(bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) || bytes.len() < 13 {
        return Err(CtfError::InvalidInput(
            "input is not a GIF file".to_string(),
        ));
    }

    let mut offset = 13;
    let packed = bytes[10];
    if packed & 0x80 != 0 {
        let table_size = 3 * (1usize << ((packed & 0x07) + 1));
        offset += table_size;
    }

    let mut frames = Vec::new();
    let mut delay_ms = 0u16;
    while offset < bytes.len() {
        match bytes[offset] {
            0x21 => {
                offset += 1;
                if offset >= bytes.len() {
                    break;
                }
                let label = bytes[offset];
                offset += 1;
                if label == 0xf9 && bytes.get(offset) == Some(&4) && offset + 5 < bytes.len() {
                    delay_ms = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]) * 10;
                }
                offset = skip_gif_sub_blocks(bytes, offset)?;
            }
            0x2c => {
                if offset + 9 >= bytes.len() {
                    break;
                }
                let x = u16::from_le_bytes([bytes[offset + 1], bytes[offset + 2]]);
                let y = u16::from_le_bytes([bytes[offset + 3], bytes[offset + 4]]);
                let width = u16::from_le_bytes([bytes[offset + 5], bytes[offset + 6]]);
                let height = u16::from_le_bytes([bytes[offset + 7], bytes[offset + 8]]);
                let image_packed = bytes[offset + 9];
                offset += 10;
                if image_packed & 0x80 != 0 {
                    offset += 3 * (1usize << ((image_packed & 0x07) + 1));
                }
                if offset >= bytes.len() {
                    break;
                }
                offset += 1;
                offset = skip_gif_sub_blocks(bytes, offset)?;
                frames.push(serde_json::json!({
                    "index": frames.len(),
                    "x": x,
                    "y": y,
                    "width": width,
                    "height": height,
                    "delay_ms": delay_ms,
                }));
                delay_ms = 0;
            }
            0x3b => break,
            _ => break,
        }
    }
    Ok(frames)
}

fn skip_gif_sub_blocks(bytes: &[u8], mut offset: usize) -> Result<usize> {
    loop {
        let Some(size) = bytes.get(offset).copied() else {
            return Err(CtfError::InvalidInput(
                "truncated GIF sub-block".to_string(),
            ));
        };
        offset += 1;
        if size == 0 {
            return Ok(offset);
        }
        offset += size as usize;
        if offset > bytes.len() {
            return Err(CtfError::InvalidInput(
                "truncated GIF sub-block".to_string(),
            ));
        }
    }
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

#[derive(Debug, Clone, Copy)]
enum Endian {
    Little,
    Big,
}

#[derive(Debug, Clone)]
struct PcapPacket {
    linktype: u32,
    data: Vec<u8>,
}

#[derive(Debug, Clone)]
struct Ipv4Payload<'a> {
    src: String,
    dst: String,
    protocol: u8,
    payload: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
struct UdpSegment<'a> {
    src_port: u16,
    dst_port: u16,
    payload: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
struct TcpSegment<'a> {
    src_port: u16,
    dst_port: u16,
    payload: &'a [u8],
}

#[derive(Debug, Clone)]
struct TcpFlowSummary {
    key: String,
    src: String,
    dst: String,
    src_port: u16,
    dst_port: u16,
    packets: usize,
    payload_bytes: usize,
}

#[derive(Debug, Clone)]
struct HidKey {
    name: String,
    text: String,
}

fn parse_pcap_packets(bytes: &[u8]) -> Result<Vec<PcapPacket>> {
    if bytes.len() < 24 {
        return Err(CtfError::InvalidInput(
            "pcap is shorter than 24 bytes".to_string(),
        ));
    }
    let endian = match bytes.get(0..4) {
        Some([0xd4, 0xc3, 0xb2, 0xa1]) | Some([0x4d, 0x3c, 0xb2, 0xa1]) => Endian::Little,
        Some([0xa1, 0xb2, 0xc3, 0xd4]) | Some([0xa1, 0xb2, 0x3c, 0x4d]) => Endian::Big,
        _ => {
            return Err(CtfError::InvalidInput(
                "unsupported pcap magic; pcapng is planned for the Python plugin path".to_string(),
            ));
        }
    };
    let linktype = read_u32(bytes, 20, endian).unwrap_or(1);
    let mut offset = 24;
    let mut packets = Vec::new();
    while offset + 16 <= bytes.len() {
        let Some(incl_len) = read_u32(bytes, offset + 8, endian).map(|value| value as usize) else {
            break;
        };
        offset += 16;
        if offset + incl_len > bytes.len() {
            break;
        }
        packets.push(PcapPacket {
            linktype,
            data: bytes[offset..offset + incl_len].to_vec(),
        });
        offset += incl_len;
    }
    Ok(packets)
}

fn parse_ipv4_payload(data: &[u8], linktype: u32) -> Option<Ipv4Payload<'_>> {
    let ip_offset = if linktype == 1 && data.len() >= 14 {
        let ethertype = u16::from_be_bytes([data[12], data[13]]);
        if ethertype != 0x0800 {
            return None;
        }
        14
    } else if data.first().map(|byte| byte >> 4) == Some(4) {
        0
    } else {
        return None;
    };
    if data.len() < ip_offset + 20 {
        return None;
    }
    let header = &data[ip_offset..];
    let ihl = ((header[0] & 0x0f) as usize) * 4;
    if ihl < 20 || header.len() < ihl {
        return None;
    }
    let total_len = u16::from_be_bytes([header[2], header[3]]) as usize;
    let packet_len = total_len.min(header.len());
    if packet_len < ihl {
        return None;
    }
    let src = ip_string(&header[12..16]);
    let dst = ip_string(&header[16..20]);
    Some(Ipv4Payload {
        src,
        dst,
        protocol: header[9],
        payload: &header[ihl..packet_len],
    })
}

fn parse_udp_segment(data: &[u8]) -> Option<UdpSegment<'_>> {
    if data.len() < 8 {
        return None;
    }
    let length = u16::from_be_bytes([data[4], data[5]]) as usize;
    let end = length.min(data.len());
    Some(UdpSegment {
        src_port: u16::from_be_bytes([data[0], data[1]]),
        dst_port: u16::from_be_bytes([data[2], data[3]]),
        payload: &data[8..end],
    })
}

fn parse_tcp_segment(data: &[u8]) -> Option<TcpSegment<'_>> {
    if data.len() < 20 {
        return None;
    }
    let header_len = ((data[12] >> 4) as usize) * 4;
    if header_len < 20 || data.len() < header_len {
        return None;
    }
    Some(TcpSegment {
        src_port: u16::from_be_bytes([data[0], data[1]]),
        dst_port: u16::from_be_bytes([data[2], data[3]]),
        payload: &data[header_len..],
    })
}

fn parse_dns_queries(payload: &[u8]) -> Vec<String> {
    if payload.len() < 12 {
        return Vec::new();
    }
    let qdcount = u16::from_be_bytes([payload[4], payload[5]]) as usize;
    let mut offset = 12;
    let mut queries = Vec::new();
    for _ in 0..qdcount {
        if let Some((name, next)) = parse_dns_name(payload, offset) {
            queries.push(name);
            offset = next.saturating_add(4);
        } else {
            break;
        }
    }
    queries
}

fn parse_dns_name(payload: &[u8], mut offset: usize) -> Option<(String, usize)> {
    let mut labels = Vec::new();
    let mut jumps = 0;
    loop {
        let len = *payload.get(offset)?;
        if len == 0 {
            offset += 1;
            break;
        }
        if len & 0xc0 == 0xc0 {
            if jumps > 4 {
                return None;
            }
            let next = *payload.get(offset + 1)? as usize;
            let pointer = (((len & 0x3f) as usize) << 8) | next;
            offset = pointer;
            jumps += 1;
            continue;
        }
        offset += 1;
        let end = offset + len as usize;
        let label = payload.get(offset..end)?;
        labels.push(String::from_utf8_lossy(label).into_owned());
        offset = end;
    }
    Some((labels.join("."), offset))
}

fn decode_usb_hid_reports(bytes: &[u8]) -> Vec<HidKey> {
    bytes
        .chunks(8)
        .filter(|chunk| chunk.len() == 8)
        .flat_map(|report| {
            let shifted = report[0] & 0x22 != 0;
            report[2..8]
                .iter()
                .filter(|keycode| **keycode != 0)
                .filter_map(move |keycode| hid_key(*keycode, shifted))
        })
        .collect()
}

fn hid_key(code: u8, shifted: bool) -> Option<HidKey> {
    let (name, normal, shifted_text) = match code {
        0x04..=0x1d => {
            let letter = (b'a' + code - 0x04) as char;
            let text = if shifted {
                letter.to_ascii_uppercase()
            } else {
                letter
            };
            return Some(HidKey {
                name: text.to_string(),
                text: text.to_string(),
            });
        }
        0x1e => ("1", "1", "!"),
        0x1f => ("2", "2", "@"),
        0x20 => ("3", "3", "#"),
        0x21 => ("4", "4", "$"),
        0x22 => ("5", "5", "%"),
        0x23 => ("6", "6", "^"),
        0x24 => ("7", "7", "&"),
        0x25 => ("8", "8", "*"),
        0x26 => ("9", "9", "("),
        0x27 => ("0", "0", ")"),
        0x28 => ("enter", "\n", "\n"),
        0x2c => ("space", " ", " "),
        0x2d => ("minus", "-", "_"),
        0x2e => ("equals", "=", "+"),
        0x2f => ("left_bracket", "[", "{"),
        0x30 => ("right_bracket", "]", "}"),
        0x31 => ("backslash", "\\", "|"),
        0x33 => ("semicolon", ";", ":"),
        0x34 => ("quote", "'", "\""),
        0x36 => ("comma", ",", "<"),
        0x37 => ("dot", ".", ">"),
        0x38 => ("slash", "/", "?"),
        _ => return None,
    };
    Some(HidKey {
        name: name.to_string(),
        text: if shifted { shifted_text } else { normal }.to_string(),
    })
}

fn parse_hex_or_raw(text: &str) -> Vec<u8> {
    let cleaned = text.trim().replace([' ', '\n', '\r', '\t', ':', '-'], "");
    if cleaned.len() >= 2
        && cleaned.len().is_multiple_of(2)
        && cleaned.chars().all(|char| char.is_ascii_hexdigit())
        && let Ok(bytes) = hex::decode(cleaned)
    {
        return bytes;
    }
    text.as_bytes().to_vec()
}

fn ip_string(bytes: &[u8]) -> String {
    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
}

fn read_u32(bytes: &[u8], offset: usize, endian: Endian) -> Option<u32> {
    let data: [u8; 4] = bytes.get(offset..offset + 4)?.try_into().ok()?;
    Some(match endian {
        Endian::Little => u32::from_le_bytes(data),
        Endian::Big => u32::from_be_bytes(data),
    })
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

    #[test]
    fn entropy_map_reports_chunks() {
        let response = file_entropy_map(&dummy_spec(), &request("file.entropy_map", "flag"))
            .expect("entropy map");
        assert!(response.outputs[0].value.contains("\"chunk_size\": 256"));
    }

    #[test]
    fn image_data_uri_detects_gif() {
        let response =
            image_base64_data_uri(&dummy_spec(), &request("image.base64.data_uri", "GIF89a"))
                .expect("data uri");
        assert!(
            response.outputs[0]
                .value
                .starts_with("data:image/gif;base64,")
        );
    }

    #[test]
    fn gif_frame_parser_counts_frame() {
        let gif = b"GIF89a\x01\x00\x01\x00\x80\x00\x00\x00\x00\x00\xff\xff\xff,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02D\x01\x00;";
        let frames = parse_gif_frames(gif).expect("gif frames");
        assert_eq!(frames.len(), 1);
    }

    #[test]
    fn usb_hid_decodes_text() {
        let response =
            usb_hid_keys(&dummy_spec(), &request("usb.hid.keys", "0000040000000000")).expect("hid");
        assert!(response.outputs[0].value.contains("\"text\": \"a\""));
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
