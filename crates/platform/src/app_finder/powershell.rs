#[cfg(windows)]
pub(super) fn decode_windows_powershell_output(bytes: &[u8]) -> String {
    let bytes = strip_utf8_bom(bytes);
    if bytes.is_empty() {
        return String::new();
    }

    if looks_like_utf16le(bytes) {
        let utf16_bytes = bytes.strip_prefix(&[0xFF, 0xFE]).unwrap_or(bytes);
        let utf16 = utf16_bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        if let Ok(decoded) = String::from_utf16(&utf16) {
            return decoded;
        }
    }

    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(windows)]
fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes)
}

#[cfg(windows)]
fn looks_like_utf16le(bytes: &[u8]) -> bool {
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        return true;
    }

    if bytes.len() < 4 {
        return false;
    }

    let sample = bytes.len().min(128);
    let mut zero_bytes = 0usize;
    let mut inspected_pairs = 0usize;
    for chunk in bytes[..sample].chunks(2) {
        if chunk.len() < 2 {
            break;
        }
        inspected_pairs += 1;
        if chunk[1] == 0 {
            zero_bytes += 1;
        }
    }

    inspected_pairs >= 4 && zero_bytes * 2 >= inspected_pairs
}
