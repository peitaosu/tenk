use serde_json::Value;

pub fn value_as_f64(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

pub fn value_as_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_f64().map(|n| n.round() as i64))
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
        .map(normalize_unix_timestamp)
}

pub fn normalize_unix_timestamp(timestamp: i64) -> i64 {
    if timestamp > 1_000_000_000_000 {
        timestamp / 1000
    } else {
        timestamp
    }
}

pub fn format_packet(content: &str) -> String {
    format!("~m~{}~m~{}", content.len(), content)
}

pub fn format_json(value: &Value) -> String {
    format_packet(&value.to_string())
}

pub fn format_message(message_type: &str, params: &[Value]) -> String {
    format_json(&serde_json::json!({ "m": message_type, "p": params }))
}

pub fn format_ping_response(seq: i64) -> String {
    format_packet(&format!("~h~{seq}"))
}

pub fn parse_packets(raw: &str) -> Vec<Value> {
    let cleaned = raw.replace("~h~", "");
    let mut packets = Vec::new();
    let mut rest = cleaned.as_str();

    while let Some(start) = rest.find("~m~") {
        rest = &rest[start + 3..];
        let Some(end) = rest.find("~m~") else {
            break;
        };
        let len_str = &rest[..end];
        let Ok(len) = len_str.parse::<usize>() else {
            rest = &rest[end + 3..];
            continue;
        };
        rest = &rest[end + 3..];
        if rest.len() < len {
            break;
        }
        let chunk = &rest[..len];
        rest = &rest[len..];
        if chunk.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(chunk) {
            packets.push(value);
        } else if let Ok(num) = chunk.parse::<i64>() {
            packets.push(Value::from(num));
        }
    }

    packets
}

pub fn parse_compressed(data: &str) -> Result<Value, crate::error::DataError> {
    use base64::Engine;
    use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
    use std::io::Read;
    use zip::ZipArchive;

    let normalised = data.replace('-', "+").replace('_', "/");
    let pad = (4 - normalised.len() % 4) % 4;
    let padded = format!("{normalised}{}", "=".repeat(pad));
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(padded)
        .map_err(|e| crate::error::DataError::custom(format!("base64 decode: {e}")))?;

    if let Ok(mut archive) = ZipArchive::new(std::io::Cursor::new(decoded.clone())) {
        for index in 0..archive.len() {
            if let Ok(mut file) = archive.by_index(index) {
                let mut text = String::new();
                if file.read_to_string(&mut text).is_ok() {
                    if let Ok(value) = serde_json::from_str(&text) {
                        return Ok(value);
                    }
                }
            }
        }
    }

    let attempts: Vec<Box<dyn Read>> = vec![
        Box::new(std::io::Cursor::new(decoded.clone())),
        Box::new(ZlibDecoder::new(decoded.as_slice())),
        Box::new(GzDecoder::new(decoded.as_slice())),
        Box::new(DeflateDecoder::new(decoded.as_slice())),
    ];

    for mut reader in attempts {
        let mut text = String::new();
        if reader.read_to_string(&mut text).is_ok() {
            if let Ok(value) = serde_json::from_str(&text) {
                return Ok(value);
            }
        }
    }

    Err(crate::error::DataError::custom(
        "unable to decompress strategy report",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_and_parse_roundtrip() {
        let msg = format_message("set_auth_token", &[Value::from("unauthorized_user_token")]);
        let packets = parse_packets(&msg);
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0]["m"], "set_auth_token");
    }

    #[test]
    fn test_value_as_i64_from_float() {
        assert_eq!(value_as_i64(&Value::from(1_704_067_200.0)), Some(1_704_067_200));
    }

    #[test]
    fn test_normalize_unix_timestamp_millis() {
        assert_eq!(normalize_unix_timestamp(1_704_067_200_000), 1_704_067_200);
    }
}
