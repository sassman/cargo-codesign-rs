//! macOS Bookmark binary format — encode and decode.
//!
//! The bookmark format is little-endian throughout. Structure:
//!   - 64-byte header (magic "book", size, version, `header_size`, security cookie, team id, reserved)
//!   - Payload: 4-byte first-TOC-offset + data items + TOC
//!
//! Data items are: len(u32 LE) + type(u32 LE) + data + pad-to-4.
//! TOC: size(u32) + sentinel(u32) + id(u32) + next(u32) + count(u32) + entries.
//! Each TOC entry: key(u32) + `data_offset(u32)` + flags(u32).

use super::types::{BinaryDecode, BinaryEncode, DecodeError};

/// Bookmark item type constants (little-endian on the wire).
const TYPE_STRING: u32 = 0x0101;
const TYPE_RAW: u32 = 0x0201;
const TYPE_U32: u32 = 0x0303;
const TYPE_U64: u32 = 0x0304;
const TYPE_F64: u32 = 0x0400;
const TYPE_BOOL: u32 = 0x0501;
const TYPE_ARRAY: u32 = 0x0601;
const TYPE_URL: u32 = 0x0901;

/// Header constants.
const HEADER_SIZE: u32 = 64;
const BOOKMARK_MAGIC: &[u8; 4] = b"book";
const BOOKMARK_VERSION: u32 = 0x1005_0000;
const TOC_SENTINEL: u32 = 0xFFFF_FFFE;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Bookmark {
    pub(crate) path_components: Vec<String>,
    pub(crate) volume_name: String,
    pub(crate) volume_path: String,
    pub(crate) volume_url: String,
    pub(crate) volume_uuid: String,
    pub(crate) volume_capacity: u64,
}

// ---------------------------------------------------------------------------
// Encode
// ---------------------------------------------------------------------------

impl BinaryEncode for Bookmark {
    #[allow(clippy::cast_possible_truncation)]
    fn encode(&self) -> Vec<u8> {
        let mut payload = Vec::with_capacity(1024);

        // Reserve space for first-TOC-offset (patched later).
        payload.extend_from_slice(&[0u8; 4]);

        // Track (key, data_offset) pairs for the TOC.
        let mut toc_entries: Vec<(u32, u32)> = Vec::new();

        // --- Data items (order matches old build_background_bookmark exactly) ---

        // CreationOptions (0xd010) = 0x20000200
        let creation_opts_off = append_u32_item(&mut payload, 0x2000_0200);
        toc_entries.push((0xd010, creation_opts_off));

        // PathComponents (0x1004): array of string offsets
        let component_offsets: Vec<u32> = self
            .path_components
            .iter()
            .map(|s| append_string(&mut payload, s))
            .collect();
        let path_arr_off = append_array(&mut payload, &component_offsets);
        toc_entries.push((0x1004, path_arr_off));

        // Inode components (0x1005): array of 4 u64(0) — all pointing to the same item
        let inode_0 = append_item(&mut payload, TYPE_U64, &0u64.to_le_bytes());
        let inode_arr_off = append_array(&mut payload, &[inode_0, inode_0, inode_0, inode_0]);
        toc_entries.push((0x1005, inode_arr_off));

        // PropertyFlags (0x1010) — fixed 24-byte blob
        let prop_flags_data: [u8; 24] = [
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let prop_off = append_item(&mut payload, TYPE_RAW, &prop_flags_data);
        toc_entries.push((0x1010, prop_off));

        // CreationDate (0x1040) — f64 0.0
        let cdate_off = append_item(&mut payload, TYPE_F64, &0.0f64.to_le_bytes());
        toc_entries.push((0x1040, cdate_off));

        // Volume attribute array (0x2000): [0xF000, 0, 1, 0, 0]
        let va_f000 = append_u32_item(&mut payload, 0xF000);
        let va_zero = append_u32_item(&mut payload, 0);
        let va_one = append_u32_item(&mut payload, 1);
        let va_arr_off = append_array(&mut payload, &[va_f000, va_zero, va_one, va_zero, va_zero]);
        toc_entries.push((0x2000, va_arr_off));

        // VolumePath (0x2002)
        let vol_path_off = append_string(&mut payload, &self.volume_path);
        toc_entries.push((0x2002, vol_path_off));

        // VolumeURL (0x2005)
        let vol_url_off = append_item(&mut payload, TYPE_URL, self.volume_url.as_bytes());
        toc_entries.push((0x2005, vol_url_off));

        // VolumeName (0x2010)
        let vol_name_off = append_string(&mut payload, &self.volume_name);
        toc_entries.push((0x2010, vol_name_off));

        // VolumeUUID (0x2011)
        let vol_uuid_off = append_string(&mut payload, &self.volume_uuid);
        toc_entries.push((0x2011, vol_uuid_off));

        // VolumeCapacity (0x2012)
        let vol_cap_off = append_item(&mut payload, TYPE_U64, &self.volume_capacity.to_le_bytes());
        toc_entries.push((0x2012, vol_cap_off));

        // VolCreationDate (0x2013) — f64 0.0
        let vol_cdate_off = append_item(&mut payload, TYPE_F64, &0.0f64.to_le_bytes());
        toc_entries.push((0x2013, vol_cdate_off));

        // VolPropertyFlags (0x2020) — fixed 24-byte blob
        let vol_prop_data: [u8; 24] = [
            0x65, 0x02, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xef, 0x13, 0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let vol_prop_off = append_item(&mut payload, TYPE_RAW, &vol_prop_data);
        toc_entries.push((0x2020, vol_prop_off));

        // key 0x2040 — u32 value 4000
        let v2040_off = append_u32_item(&mut payload, 4000);
        toc_entries.push((0x2040, v2040_off));

        // key 0xd001 — BoolTrue (empty data)
        let bool_true_off = append_item(&mut payload, TYPE_BOOL, &[]);
        toc_entries.push((0xd001, bool_true_off));

        // Sort TOC entries by key (required for binary search by Finder).
        toc_entries.sort_by_key(|&(key, _)| key);

        // --- Build TOC ---
        let first_toc_offset = payload.len() as u32;

        let toc_body_size = 4 * 4 + toc_entries.len() as u32 * 12;
        payload.extend_from_slice(&toc_body_size.to_le_bytes());
        payload.extend_from_slice(&TOC_SENTINEL.to_le_bytes());
        payload.extend_from_slice(&1u32.to_le_bytes()); // id
        payload.extend_from_slice(&0u32.to_le_bytes()); // next_toc (none)
        payload.extend_from_slice(&(toc_entries.len() as u32).to_le_bytes());

        for &(key, data_off) in &toc_entries {
            payload.extend_from_slice(&key.to_le_bytes());
            payload.extend_from_slice(&data_off.to_le_bytes());
            payload.extend_from_slice(&0u32.to_le_bytes()); // flags
        }

        // Patch first-TOC-offset at the start of payload.
        payload[0..4].copy_from_slice(&first_toc_offset.to_le_bytes());

        // --- Build header ---
        let total_size = HEADER_SIZE + payload.len() as u32;

        let mut bookmark = Vec::with_capacity(total_size as usize);
        bookmark.extend_from_slice(BOOKMARK_MAGIC);
        bookmark.extend_from_slice(&total_size.to_le_bytes());
        bookmark.extend_from_slice(&BOOKMARK_VERSION.to_le_bytes());
        bookmark.extend_from_slice(&HEADER_SIZE.to_le_bytes());
        bookmark.extend_from_slice(&[0u8; 32]); // security cookie
        bookmark.extend_from_slice(b"0000000000"); // team id (10 bytes)
        bookmark.extend_from_slice(&[0u8; 6]); // reserved

        bookmark.extend_from_slice(&payload);
        bookmark
    }
}

// ---------------------------------------------------------------------------
// Payload item helpers
// ---------------------------------------------------------------------------

/// Append a data item to the payload, returning its offset from payload start.
#[allow(clippy::cast_possible_truncation)]
fn append_item(payload: &mut Vec<u8>, item_type: u32, data: &[u8]) -> u32 {
    let offset = payload.len() as u32;
    payload.extend_from_slice(&(data.len() as u32).to_le_bytes());
    payload.extend_from_slice(&item_type.to_le_bytes());
    payload.extend_from_slice(data);
    // Pad to 4-byte alignment.
    let pad = (4 - (data.len() % 4)) % 4;
    for _ in 0..pad {
        payload.push(0);
    }
    offset
}

fn append_string(payload: &mut Vec<u8>, s: &str) -> u32 {
    append_item(payload, TYPE_STRING, s.as_bytes())
}

#[allow(clippy::cast_possible_truncation)]
fn append_u32_item(payload: &mut Vec<u8>, v: u32) -> u32 {
    append_item(payload, TYPE_U32, &v.to_le_bytes())
}

fn append_array(payload: &mut Vec<u8>, offsets: &[u32]) -> u32 {
    let data: Vec<u8> = offsets.iter().flat_map(|o| o.to_le_bytes()).collect();
    append_item(payload, TYPE_ARRAY, &data)
}

// ---------------------------------------------------------------------------
// Decode
// ---------------------------------------------------------------------------

impl BinaryDecode for Bookmark {
    #[allow(clippy::cast_possible_truncation)]
    fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        // Validate header.
        if data.len() < HEADER_SIZE as usize {
            return Err(DecodeError::TooShort {
                expected: HEADER_SIZE as usize,
                got: data.len(),
            });
        }

        if &data[0..4] != BOOKMARK_MAGIC {
            return Err(DecodeError::InvalidMagic {
                expected: BOOKMARK_MAGIC,
                got: data[0..4].to_vec(),
            });
        }

        let total_size = read_u32_le(data, 4) as usize;
        if data.len() < total_size {
            return Err(DecodeError::TooShort {
                expected: total_size,
                got: data.len(),
            });
        }

        let header_size = read_u32_le(data, 12) as usize;
        let payload = &data[header_size..total_size];

        // First 4 bytes of payload = first TOC offset (relative to payload start).
        if payload.len() < 4 {
            return Err(DecodeError::TooShort {
                expected: 4,
                got: payload.len(),
            });
        }
        let first_toc_off = read_u32_le(payload, 0) as usize;

        // Parse TOC.
        let toc = &payload[first_toc_off..];
        if toc.len() < 20 {
            return Err(DecodeError::TooShort {
                expected: 20,
                got: toc.len(),
            });
        }
        // Skip: toc_body_size(4) + sentinel(4) + id(4) + next(4)
        let entry_count = read_u32_le(toc, 16) as usize;
        let entries_start = 20;

        if toc.len() < entries_start + entry_count * 12 {
            return Err(DecodeError::TooShort {
                expected: entries_start + entry_count * 12,
                got: toc.len(),
            });
        }

        // Build a key -> data_offset map from the TOC.
        let mut key_map = std::collections::BTreeMap::new();
        for i in 0..entry_count {
            let base = entries_start + i * 12;
            let key = read_u32_le(toc, base);
            let data_off = read_u32_le(toc, base + 4);
            key_map.insert(key, data_off);
        }

        // Extract configurable fields from the payload using the TOC offsets.
        let path_components = decode_path_components(payload, &key_map)?;
        let volume_name = decode_string_key(payload, &key_map, 0x2010, "VolumeName")?;
        let volume_path = decode_string_key(payload, &key_map, 0x2002, "VolumePath")?;
        let volume_url = decode_url_or_string_key(payload, &key_map, 0x2005, "VolumeURL")?;
        let volume_uuid = decode_string_key(payload, &key_map, 0x2011, "VolumeUUID")?;
        let volume_capacity = decode_u64_key(payload, &key_map, 0x2012, "VolumeCapacity")?;

        Ok(Bookmark {
            path_components,
            volume_name,
            volume_path,
            volume_url,
            volume_uuid,
            volume_capacity,
        })
    }
}

// ---------------------------------------------------------------------------
// Decode helpers
// ---------------------------------------------------------------------------

fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        data[offset..offset + 4]
            .try_into()
            .expect("slice is exactly 4 bytes"),
    )
}

fn read_u64_le(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        data[offset..offset + 8]
            .try_into()
            .expect("slice is exactly 8 bytes"),
    )
}

/// Read the data item at `offset` within the payload.
/// Returns `(item_type, item_data_slice)`.
fn read_item(payload: &[u8], offset: u32) -> Result<(u32, &[u8]), DecodeError> {
    let off = offset as usize;
    if off + 8 > payload.len() {
        return Err(DecodeError::TooShort {
            expected: off + 8,
            got: payload.len(),
        });
    }
    let len = read_u32_le(payload, off) as usize;
    let item_type = read_u32_le(payload, off + 4);
    let data_start = off + 8;
    let data_end = data_start + len;
    if data_end > payload.len() {
        return Err(DecodeError::TooShort {
            expected: data_end,
            got: payload.len(),
        });
    }
    Ok((item_type, &payload[data_start..data_end]))
}

/// Decode key 0x1004 (PathComponents): an array of string offsets.
fn decode_path_components(
    payload: &[u8],
    key_map: &std::collections::BTreeMap<u32, u32>,
) -> Result<Vec<String>, DecodeError> {
    let &arr_off = key_map
        .get(&0x1004)
        .ok_or_else(|| DecodeError::Other("missing TOC key 0x1004 (PathComponents)".into()))?;

    let (item_type, arr_data) = read_item(payload, arr_off)?;
    if item_type != TYPE_ARRAY {
        return Err(DecodeError::Other(format!(
            "expected array type 0x0601 for PathComponents, got 0x{item_type:04x}"
        )));
    }

    let count = arr_data.len() / 4;
    let mut components = Vec::with_capacity(count);
    for i in 0..count {
        let str_off = read_u32_le(arr_data, i * 4);
        let (str_type, str_data) = read_item(payload, str_off)?;
        if str_type != TYPE_STRING {
            return Err(DecodeError::Other(format!(
                "expected string type 0x0101 in PathComponents, got 0x{str_type:04x}"
            )));
        }
        let s = String::from_utf8(str_data.to_vec())
            .map_err(|e| DecodeError::Other(format!("invalid UTF-8 in PathComponents: {e}")))?;
        components.push(s);
    }
    Ok(components)
}

/// Decode a string item at the given TOC key.
fn decode_string_key(
    payload: &[u8],
    key_map: &std::collections::BTreeMap<u32, u32>,
    key: u32,
    label: &str,
) -> Result<String, DecodeError> {
    let &off = key_map
        .get(&key)
        .ok_or_else(|| DecodeError::Other(format!("missing TOC key 0x{key:04x} ({label})")))?;

    let (item_type, item_data) = read_item(payload, off)?;
    if item_type != TYPE_STRING {
        return Err(DecodeError::Other(format!(
            "expected string type for {label}, got 0x{item_type:04x}"
        )));
    }
    String::from_utf8(item_data.to_vec())
        .map_err(|e| DecodeError::Other(format!("invalid UTF-8 in {label}: {e}")))
}

/// Decode a URL (0x0901) or string (0x0101) item at the given TOC key.
fn decode_url_or_string_key(
    payload: &[u8],
    key_map: &std::collections::BTreeMap<u32, u32>,
    key: u32,
    label: &str,
) -> Result<String, DecodeError> {
    let &off = key_map
        .get(&key)
        .ok_or_else(|| DecodeError::Other(format!("missing TOC key 0x{key:04x} ({label})")))?;

    let (item_type, item_data) = read_item(payload, off)?;
    if item_type != TYPE_URL && item_type != TYPE_STRING {
        return Err(DecodeError::Other(format!(
            "expected URL or string type for {label}, got 0x{item_type:04x}"
        )));
    }
    String::from_utf8(item_data.to_vec())
        .map_err(|e| DecodeError::Other(format!("invalid UTF-8 in {label}: {e}")))
}

/// Decode a u64 item at the given TOC key.
fn decode_u64_key(
    payload: &[u8],
    key_map: &std::collections::BTreeMap<u32, u32>,
    key: u32,
    label: &str,
) -> Result<u64, DecodeError> {
    let &off = key_map
        .get(&key)
        .ok_or_else(|| DecodeError::Other(format!("missing TOC key 0x{key:04x} ({label})")))?;

    let (item_type, item_data) = read_item(payload, off)?;
    if item_type != TYPE_U64 {
        return Err(DecodeError::Other(format!(
            "expected u64 type for {label}, got 0x{item_type:04x}"
        )));
    }
    if item_data.len() < 8 {
        return Err(DecodeError::TooShort {
            expected: 8,
            got: item_data.len(),
        });
    }
    Ok(read_u64_le(item_data, 0))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_bookmark() -> Bookmark {
        Bookmark {
            path_components: vec![
                "Volumes".to_string(),
                "JPEG Locker".to_string(),
                ".background".to_string(),
                "bg.png".to_string(),
            ],
            volume_name: "JPEG Locker".to_string(),
            volume_path: "/Volumes/JPEG Locker".to_string(),
            volume_url: "file:///Volumes/JPEG Locker/".to_string(),
            volume_uuid: "00000000-0000-0000-0000-000000000000".to_string(),
            volume_capacity: 52_428_800,
        }
    }

    #[test]
    fn encode_starts_with_book_magic() {
        let encoded = test_bookmark().encode();

        // Magic bytes.
        assert_eq!(
            &encoded[..4],
            b"book",
            "bookmark must start with 'book' magic"
        );

        // Total size field matches actual length.
        let total_size = u32::from_le_bytes(encoded[4..8].try_into().unwrap()) as usize;
        assert_eq!(
            total_size,
            encoded.len(),
            "total_size header field must equal actual byte length"
        );
    }

    #[test]
    fn toc_has_valid_structure() {
        let encoded = test_bookmark().encode();
        let header_size = u32::from_le_bytes(encoded[12..16].try_into().unwrap()) as usize;
        let payload = &encoded[header_size..];

        // First TOC offset.
        let first_toc_off = u32::from_le_bytes(payload[0..4].try_into().unwrap()) as usize;
        let toc = &payload[first_toc_off..];

        // Sentinel.
        let sentinel = u32::from_le_bytes(toc[4..8].try_into().unwrap());
        assert_eq!(sentinel, 0xFFFF_FFFE, "TOC sentinel must be 0xFFFFFFFE");

        // Entry count = 15 (matches old code's 15 TOC entries).
        let count = u32::from_le_bytes(toc[16..20].try_into().unwrap());
        assert_eq!(count, 15, "TOC must have 15 entries");

        // Entries must be sorted by key.
        let mut prev_key = 0u32;
        for i in 0..count as usize {
            let base = 20 + i * 12;
            let key = u32::from_le_bytes(toc[base..base + 4].try_into().unwrap());
            assert!(
                key >= prev_key,
                "TOC keys must be sorted: 0x{prev_key:04x} > 0x{key:04x}"
            );
            prev_key = key;
        }
    }

    #[test]
    fn encode_decode_roundtrip() {
        let original = test_bookmark();
        let encoded = original.encode();
        let decoded = Bookmark::decode(&encoded).expect("decode must succeed");

        assert_eq!(decoded.path_components, original.path_components);
        assert_eq!(decoded.volume_name, original.volume_name);
        assert_eq!(decoded.volume_path, original.volume_path);
        assert_eq!(decoded.volume_url, original.volume_url);
        assert_eq!(decoded.volume_uuid, original.volume_uuid);
        assert_eq!(decoded.volume_capacity, original.volume_capacity);
        assert_eq!(decoded, original);
    }
}
