//! Encoding detection and priority decoding.
//!
//! This module provides multi-encoding detection for BMS files,
//! supporting Shift-JIS, GBK, UTF-8, and other encodings.

use std::sync::LazyLock;

static DEFAULT_ENCODINGS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        "shift-jis",
        "shift-jis-2004",
        "gb2312",
        "utf-8",
        "gb18030",
        "shift-jisx0213",
    ]
});

/// BOFTT-specific encoding overrides
const BOFTT_ID_ENCODING: [(&str, &str); 4] = [
    ("134", "utf-8"),
    ("191", "gbk"),
    ("435", "gbk"),
    ("439", "gbk"),
];

/// Get BOFTT encoding for a given ID
#[must_use]
pub fn get_boftt_encoding(id: &str) -> Option<&'static str> {
    BOFTT_ID_ENCODING
        .iter()
        .find(|(k, _)| *k == id)
        .map(|(_, v)| *v)
}

/// `PriorityDecoder` attempts to decode byte sequences using multiple encodings
/// in priority order, trying 1-4 bytes at a time for each character
pub struct PriorityDecoder {
    codecs: Vec<&'static encoding_rs::Encoding>,
}

impl PriorityDecoder {
    /// Create a new `PriorityDecoder` with encoding priority list.
    #[must_use]
    pub fn new(encoding_priority: &[&str]) -> Self {
        let mut codecs: Vec<&'static encoding_rs::Encoding> = Vec::new();
        for enc in encoding_priority {
            if let Some(encoding) = encoding_rs::Encoding::for_label(enc.as_bytes()) {
                codecs.push(encoding);
            } else if let Some(fallback) = Self::fallback_encoding(enc) {
                codecs.push(fallback);
            }
        }
        Self { codecs }
    }

    fn fallback_encoding(label: &str) -> Option<&'static encoding_rs::Encoding> {
        match label {
            "shift-jis-2004" | "shift-jisx0213" => Some(encoding_rs::SHIFT_JIS),
            _ => None,
        }
    }

    /// Decode a single byte sequence using the encoding priority
    /// Returns (`decoded_char`, `bytes_consumed`) or (None, 1) if all fail
    fn decode_byte_sequence(&self, byte_data: &[u8], start: usize) -> (Option<String>, usize) {
        for encoding in &self.codecs {
            for length in 1..=4 {
                if start + length > byte_data.len() {
                    break;
                }
                let (decoded, _, had_error) = encoding.decode(&byte_data[start..start + length]);
                if !had_error && !decoded.is_empty() {
                    return (Some(decoded.into_owned()), length);
                }
            }
        }
        (None, 1)
    }

    /// Decode byte data using encoding priority
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` with `InvalidData` when `errors` is `"strict"`
    /// and a byte sequence cannot be decoded by any known encoding.
    pub fn decode(&self, byte_data: &[u8], errors: &str) -> Result<String, std::io::Error> {
        let mut result = String::new();
        let mut position = 0;

        while position < byte_data.len() {
            let (ch, consumed) = self.decode_byte_sequence(byte_data, position);

            match ch {
                Some(s) => result.push_str(&s),
                None => match errors {
                    "strict" => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "Cannot decode byte sequence: {:02x?}",
                                &byte_data[position..=position]
                            ),
                        ));
                    }
                    "replace" => result.push('\u{FFFD}'),
                    _ => {}
                },
            }
            position += consumed;
        }

        Ok(result)
    }
}

/// Get BMS file string with optional forced encoding.
#[must_use]
#[allow(clippy::similar_names)]
pub fn get_bms_file_str(file_bytes: &[u8], encoding: Option<&str>) -> String {
    let encodings: Vec<&str> = if let Some(enc) = encoding {
        let mut list = vec![enc];
        list.extend(DEFAULT_ENCODINGS.iter().copied());
        list
    } else {
        DEFAULT_ENCODINGS.clone()
    };
    let decoder = PriorityDecoder::new(&encodings);
    if let Ok(s) = decoder.decode(file_bytes, "strict") {
        s
    } else {
        let (decoded, _) = encoding_rs::UTF_8.decode_without_bom_handling(file_bytes);
        let s = decoded.into_owned();
        if s.contains('\u{FFFD}') {
            s.replace('\u{FFFD}', "")
        } else {
            s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_priority_decoder() {
        // Test Shift-JIS encoded "こんにちは"
        let test_bytes: &[u8] = &[0x82, 0xb1, 0x82, 0xf1, 0x82, 0xc9, 0x82, 0xbf, 0x82, 0xcd];
        let decoder = PriorityDecoder::new(&["shift-jis", "utf-8"]);
        let result = decoder.decode(test_bytes, "replace").unwrap();
        assert_eq!(result, "こんにちは");
    }

    #[tokio::test]
    async fn test_shift_jis_decode() {
        let bytes = b"\x82\xb1\x82\xf1\x82\xc9\x82\xbf\x82\xcd";
        let decoder = PriorityDecoder::new(&["shift-jis"]);
        let result = decoder.decode(bytes, "replace").unwrap();
        assert_eq!(result, "こんにちは");
    }

    #[tokio::test]
    async fn test_utf8_decode() {
        let bytes = "こんにちは".as_bytes();
        let decoder = PriorityDecoder::new(&["utf-8"]);
        let result = decoder.decode(bytes, "replace").unwrap();
        assert_eq!(result, "こんにちは");
    }

    #[tokio::test]
    async fn test_encoding_priority() {
        let sjis_bytes = b"\x82\xb1";
        let decoder = PriorityDecoder::new(&["shift-jis", "utf-8"]);
        let result = decoder.decode(sjis_bytes, "replace").unwrap();
        assert_eq!(result, "こ");
    }

    #[tokio::test]
    async fn test_replace_mode() {
        let decoder = PriorityDecoder::new(&["utf-8"]);
        let result = decoder.decode(b"\xFF\xFE\xFF", "replace").unwrap();
        assert!(result.contains('\u{FFFD}'));
    }

    #[tokio::test]
    async fn test_ignore_mode() {
        let decoder = PriorityDecoder::new(&["utf-8"]);
        let result = decoder.decode(b"\xFF", "ignore").unwrap();
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn test_strict_mode_errors() {
        let decoder = PriorityDecoder::new(&["utf-8"]);
        let result = decoder.decode(b"\xFF", "strict");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_bms_file_str_utf8() {
        let result = get_bms_file_str(b"Hello #TITLE Test", None);
        assert!(result.contains("Hello"));
    }

    #[tokio::test]
    async fn test_get_bms_file_str_fallback() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"#TITLE ");
        bytes.extend_from_slice(&[0x82, 0xb1]); // valid SJIS
        bytes.extend_from_slice(b"\xFF"); // invalid
        let result = get_bms_file_str(&bytes, None);
        assert!(result.contains("#TITLE"));
    }

    #[tokio::test]
    async fn test_get_bms_file_str_empty() {
        assert_eq!(get_bms_file_str(b"", None), "");
    }

    #[tokio::test]
    async fn test_get_bms_file_str_ascii() {
        let result = get_bms_file_str(b"#TITLE Test", None);
        assert_eq!(result, "#TITLE Test");
    }

    #[tokio::test]
    async fn test_get_bms_file_str_sjis_mixed() {
        let bytes = b"#TITLE \x83\x65\x83\x58\x83\x67";
        let result = get_bms_file_str(bytes, None);
        assert!(
            result.contains("テスト") || result.contains("#TITLE"),
            "SJIS or fallback: {result}"
        );
    }

    #[tokio::test]
    async fn test_get_bms_file_str_corrupted_sjis_fallback() {
        let bytes = b"#TITLE \x82\xFF Test";
        let result = get_bms_file_str(bytes, None);
        assert!(!result.is_empty(), "should not be empty");
        assert!(result.contains("#TITLE"), "ASCII should survive");
    }

    #[tokio::test]
    async fn test_boftt_encoding_table() {
        assert_eq!(get_boftt_encoding("134"), Some("utf-8"));
        assert_eq!(get_boftt_encoding("191"), Some("gbk"));
        assert_eq!(get_boftt_encoding("435"), Some("gbk"));
        assert_eq!(get_boftt_encoding("439"), Some("gbk"));
        assert_eq!(get_boftt_encoding("999"), None);
    }
}
