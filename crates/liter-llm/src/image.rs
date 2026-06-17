//! Base64 data URL helpers for inline image payloads.
//!
//! Use these utilities to encode raw image bytes as `data:<mime>;base64,<b64>`
//! strings suitable for embedding in [`crate::ContentPart::ImageUrl`] without
//! a separate HTTP image host.
//!
//! # Example
//!
//! ```
//! use liter_llm::image::{encode_data_url, decode_data_url, IMAGE_PNG};
//!
//! let raw = b"fake-png-bytes";
//! let url = encode_data_url(raw, Some(IMAGE_PNG));
//! assert!(url.starts_with("data:image/png;base64,"));
//!
//! let (mime, decoded) = decode_data_url(&url).expect("valid data URL");
//! assert_eq!(mime, IMAGE_PNG);
//! assert_eq!(decoded, raw);
//! ```

use base64::Engine as _;

/// MIME type constant for PNG images.
pub const IMAGE_PNG: &str = "image/png";
/// MIME type constant for JPEG images.
pub const IMAGE_JPEG: &str = "image/jpeg";
/// MIME type constant for WebP images.
pub const IMAGE_WEBP: &str = "image/webp";
/// MIME type constant for TIFF images.
pub const IMAGE_TIFF: &str = "image/tiff";

/// Encode bytes as a base64 data URL: `data:<mime>;base64,<b64>`.
///
/// `mime` defaults to [`IMAGE_PNG`] when `None`.
///
/// # Example
///
/// ```
/// use liter_llm::image::{encode_data_url, IMAGE_PNG, IMAGE_JPEG};
///
/// let url = encode_data_url(b"\x89PNG", Some(IMAGE_PNG));
/// assert!(url.starts_with("data:image/png;base64,"));
///
/// let url_default = encode_data_url(b"\x89PNG", None);
/// assert!(url_default.starts_with("data:image/png;base64,"));
///
/// let jpeg_url = encode_data_url(b"\xff\xd8\xff", Some(IMAGE_JPEG));
/// assert!(jpeg_url.starts_with("data:image/jpeg;base64,"));
/// ```
pub fn encode_data_url(bytes: &[u8], mime: Option<&str>) -> String {
    let mime = mime.unwrap_or(IMAGE_PNG);
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:{mime};base64,{b64}")
}

/// Decode a base64 data URL into `(mime, bytes)`.
///
/// Returns `None` for:
/// - Non-data URLs (strings that do not start with `"data:"`).
/// - Malformed prefixes (missing `";base64,"` marker).
/// - Invalid base64 payloads.
///
/// The returned MIME string is extracted verbatim from the URL prefix —
/// it is not validated or normalised.
///
/// # Example
///
/// ```
/// use liter_llm::image::{encode_data_url, decode_data_url, IMAGE_PNG};
///
/// let url = encode_data_url(b"hello", Some(IMAGE_PNG));
/// let (mime, bytes) = decode_data_url(&url).expect("valid data URL");
/// assert_eq!(mime, IMAGE_PNG);
/// assert_eq!(bytes, b"hello");
///
/// // Non-data URLs return None.
/// assert!(decode_data_url("https://example.com/img.png").is_none());
///
/// // Missing ;base64, marker returns None.
/// assert!(decode_data_url("data:image/png,plaintext").is_none());
/// ```
pub fn decode_data_url(url: &str) -> Option<(String, Vec<u8>)> {
    let rest = url.strip_prefix("data:")?;
    let marker = ";base64,";
    let marker_pos = rest.find(marker)?;
    let mime = rest[..marker_pos].to_owned();
    let b64 = &rest[marker_pos + marker.len()..];
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    Some((mime, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_default_mime_is_png() {
        let url = encode_data_url(b"hi", None);
        assert!(
            url.starts_with("data:image/png;base64,"),
            "expected png prefix, got: {url}"
        );
    }

    #[test]
    fn encode_explicit_mime() {
        let url = encode_data_url(b"hi", Some(IMAGE_JPEG));
        assert!(
            url.starts_with("data:image/jpeg;base64,"),
            "expected jpeg prefix, got: {url}"
        );
    }

    #[test]
    fn decode_round_trip() {
        let payload = b"round-trip bytes \x00\x01\x02";
        for mime in [IMAGE_PNG, IMAGE_JPEG, IMAGE_WEBP, IMAGE_TIFF] {
            let url = encode_data_url(payload, Some(mime));
            let (decoded_mime, decoded_bytes) = decode_data_url(&url).unwrap_or_else(|| {
                panic!("round-trip failed for mime={mime}");
            });
            assert_eq!(decoded_mime, mime, "mime mismatch for {mime}");
            assert_eq!(decoded_bytes, payload, "bytes mismatch for {mime}");
        }
    }

    #[test]
    fn decode_rejects_non_data_url() {
        assert!(decode_data_url("https://example.com/img.png").is_none());
    }

    #[test]
    fn decode_rejects_malformed_base64() {
        assert!(decode_data_url("data:image/png;base64,!@#$").is_none());
    }

    #[test]
    fn decode_rejects_missing_base64_marker() {
        assert!(decode_data_url("data:image/png,plaintext").is_none());
    }

    #[test]
    fn byte_patterns_round_trip() {
        // Property-style coverage over a fixed table of byte patterns; the
        // proptest crate is not a workspace dev-dep.
        let test_cases: &[&[u8]] = &[
            b"",
            b"\x00",
            b"\xff\xfe\xfd",
            b"hello world",
            &[0u8; 256],
            &(0u8..=255u8).collect::<Vec<_>>(),
        ];
        for &bytes in test_cases {
            let url = encode_data_url(bytes, Some(IMAGE_PNG));
            let (mime, decoded) =
                decode_data_url(&url).unwrap_or_else(|| panic!("round-trip failed for input len={}", bytes.len()));
            assert_eq!(mime, IMAGE_PNG);
            assert_eq!(decoded, bytes);
        }
    }
}
