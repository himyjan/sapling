/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

//! Magic-byte sniffer used to skip HTTP-layer compression for blobs that are
//! already in a compressed container format (zip/apk, gzip, zstd, png, jpeg,
//! mp4, etc.). Run-time gated via JustKnob and deploy-time gated via CLI flag.

/// Number of leading bytes the sniffer needs to make a decision. Callers should
/// only invoke `looks_compressed` if at least this many bytes are available.
pub const SNIFF_PREFIX_BYTES: usize = 16;

/// If the prefix matches a known already-compressed container format, returns
/// a short stable label naming it (e.g. `"zip"`, `"gzip"`, `"png"`). Returns
/// `None` if no format matched. Labels are intended for Scuba breakdown so we
/// can attribute CPU savings back to specific blob types.
///
/// Designed to be cheap and safe to call on the hot path: a handful of byte
/// comparisons over a small slice.
pub fn looks_compressed(prefix: &[u8]) -> Option<&'static str> {
    if prefix.len() < 3 {
        return None;
    }

    let two = &prefix[..2];
    if two == [0x1f, 0x8b] {
        return Some("gzip");
    }
    if two == [0x42, 0x5a] && prefix[2] == 0x68 {
        return Some("bzip2");
    }

    if prefix.len() >= 4 {
        let four = &prefix[..4];
        // Catches .zip, .jar, .apk, .docx, .xlsx, .apex, .capex, etc. Also
        // matches `file(1)`'s "Zip archive data, at least v0.0/v2.0 to extract".
        if four == [0x50, 0x4b, 0x03, 0x04]
            || four == [0x50, 0x4b, 0x05, 0x06]
            || four == [0x50, 0x4b, 0x07, 0x08]
        {
            return Some("zip");
        }
        if four == [0x28, 0xb5, 0x2f, 0xfd] {
            return Some("zstd");
        }
        if four == [0x89, 0x50, 0x4e, 0x47] {
            return Some("png");
        }
        if four == [0x52, 0x61, 0x72, 0x21] {
            return Some("rar");
        }
        if four == [0x25, 0x50, 0x44, 0x46] {
            return Some("pdf");
        }
    }

    if prefix.len() >= 3 && prefix[..3] == [0xff, 0xd8, 0xff] {
        return Some("jpeg");
    }

    if prefix.len() >= 6 && prefix[..6] == [0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00] {
        return Some("xz");
    }
    if prefix.len() >= 6 && prefix[..6] == [0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c] {
        return Some("7z");
    }

    // ISO base media (mp4 / mov / heic / heif / avif): bytes 4..8 are b"ftyp".
    if prefix.len() >= 8 && &prefix[4..8] == b"ftyp" {
        return Some("iso_base_media");
    }

    // WebM / Matroska EBML
    if prefix.len() >= 4 && prefix[..4] == [0x1a, 0x45, 0xdf, 0xa3] {
        return Some("matroska");
    }

    None
}

#[cfg(test)]
mod test {
    use mononoke_macros::mononoke;

    use super::*;

    #[mononoke::test]
    fn detects_gzip() {
        assert_eq!(looks_compressed(&[0x1f, 0x8b, 0x08, 0x00]), Some("gzip"));
    }

    #[mononoke::test]
    fn detects_zip_apk_jar() {
        assert_eq!(
            looks_compressed(&[0x50, 0x4b, 0x03, 0x04, 0, 0, 0, 0]),
            Some("zip")
        );
        assert_eq!(
            looks_compressed(&[0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0]),
            Some("zip")
        );
        assert_eq!(
            looks_compressed(&[0x50, 0x4b, 0x07, 0x08, 0, 0, 0, 0]),
            Some("zip")
        );
    }

    #[mononoke::test]
    fn detects_zstd() {
        assert_eq!(
            looks_compressed(&[0x28, 0xb5, 0x2f, 0xfd, 0, 0, 0, 0]),
            Some("zstd")
        );
    }

    #[mononoke::test]
    fn detects_xz_bzip2_7z() {
        assert_eq!(
            looks_compressed(&[0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00]),
            Some("xz")
        );
        assert_eq!(
            looks_compressed(&[0x42, 0x5a, 0x68, 0x39, 0, 0]),
            Some("bzip2")
        );
        assert_eq!(
            looks_compressed(&[0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c]),
            Some("7z")
        );
    }

    #[mononoke::test]
    fn detects_image_formats() {
        assert_eq!(
            looks_compressed(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
            Some("png")
        );
        assert_eq!(
            looks_compressed(&[0xff, 0xd8, 0xff, 0xe0, 0, 0, 0, 0]),
            Some("jpeg")
        );
    }

    #[mononoke::test]
    fn detects_iso_base_media() {
        let mp4 = b"\x00\x00\x00\x18ftypmp42";
        assert_eq!(looks_compressed(mp4), Some("iso_base_media"));
        let heic = b"\x00\x00\x00\x18ftypheic";
        assert_eq!(looks_compressed(heic), Some("iso_base_media"));
    }

    #[mononoke::test]
    fn skips_text_and_short_inputs() {
        assert_eq!(looks_compressed(b""), None);
        assert_eq!(looks_compressed(b"hi"), None);
        assert_eq!(looks_compressed(b"hello world, this is plain text"), None);
        assert_eq!(looks_compressed(b"ELF\x02"), None);
        assert_eq!(looks_compressed(b"\x7fELF\x02\x01\x01\x00"), None);
        assert_eq!(looks_compressed(b"#!/bin/bash\n"), None);
    }
}
