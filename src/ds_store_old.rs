//! Writes a minimal `.DS_Store` file for a macOS DMG installer.
//!
//! This module generates a binary `.DS_Store` that positions icons and sets
//! window properties for a drag-to-install DMG layout. It replaces the
//! `AppleScript` + Finder approach that is slow and flaky in CI.
//!
//! The format is a buddy-allocator-based B-tree. We only ever write a single
//! leaf node with a handful of records, so the structure is fixed-layout:
//! one allocation for the DSDB root block and one for the leaf node.

use plist::Value as PlistValue;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Canonical filename for the background image inside the DMG's `.background/` folder.
/// The alias and bookmark in the `.DS_Store` reference this exact name, so the file
/// on disk must match — regardless of what the user configured as the source path.
pub const DMG_BG_FILENAME: &str = "bg.png";

/// Layout parameters for a DMG installer window.
pub struct DmgLayout {
    pub window_width: u32,
    pub window_height: u32,
    pub icon_size: u32,
    pub app_name: String,
    pub app_x: u32,
    pub app_y: u32,
    pub apps_link_x: u32,
    pub apps_link_y: u32,
    pub background_filename: String,
    pub volume_name: String,
}

// ---------------------------------------------------------------------------
// Internal enums
// ---------------------------------------------------------------------------

/// The four-byte ASCII code identifying a record's purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordCode {
    Iloc,
    Bwsp,
    Icvp,
    PBBk,
    VSrn,
}

impl RecordCode {
    fn as_bytes(self) -> [u8; 4] {
        match self {
            Self::Iloc => *b"Iloc",
            Self::Bwsp => *b"bwsp",
            Self::Icvp => *b"icvp",
            Self::PBBk => *b"pBBk",
            Self::VSrn => *b"vSrn",
        }
    }
}

/// The value payload of a `.DS_Store` record.
#[derive(Debug, Clone, PartialEq)]
enum DsStoreValue {
    Blob(Vec<u8>),
    Long(i32),
    #[allow(dead_code)]
    Bool(bool),
}

impl DsStoreValue {
    /// Four-byte type tag written to the wire format.
    fn type_code(&self) -> &[u8; 4] {
        match self {
            Self::Blob(_) => b"blob",
            Self::Long(_) => b"long",
            Self::Bool(_) => b"bool",
        }
    }

    /// Serialized payload bytes (after the type code).
    #[allow(clippy::cast_possible_truncation)]
    fn encode(&self) -> Vec<u8> {
        match self {
            Self::Blob(data) => {
                let mut out = Vec::with_capacity(4 + data.len());
                // Blob lengths in DS_Store are u32; our blobs are always < 4 GiB.
                out.extend_from_slice(&(data.len() as u32).to_be_bytes());
                out.extend_from_slice(data);
                out
            }
            Self::Long(v) => v.to_be_bytes().to_vec(),
            Self::Bool(v) => vec![u8::from(*v)],
        }
    }
}

/// A single `.DS_Store` record.
struct Record {
    filename: String,
    code: RecordCode,
    value: DsStoreValue,
}

impl Record {
    #[allow(clippy::cast_possible_truncation)]
    fn encode(&self) -> Vec<u8> {
        let utf16: Vec<u16> = self.filename.encode_utf16().collect();
        let mut out = Vec::new();

        // filename length (UTF-16 code units) + UTF-16 BE bytes
        // Filenames in our records are short ASCII; u32 truncation is safe.
        out.extend_from_slice(&(utf16.len() as u32).to_be_bytes());
        for unit in &utf16 {
            out.extend_from_slice(&unit.to_be_bytes());
        }

        // code + type + value
        out.extend_from_slice(&self.code.as_bytes());
        out.extend_from_slice(self.value.type_code());
        out.extend_from_slice(&self.value.encode());
        out
    }
}

// ---------------------------------------------------------------------------
// Block address encoding
// ---------------------------------------------------------------------------

/// Encode a block address from an offset and a size-class (log2 of size).
///
/// The `.DS_Store` block address packs the offset in the upper bits and the
/// `log2(size)` in the lower 5 bits:
///
///   `address = (offset & !0x1f) | size_class`
///
/// The offset stored here is relative to the data region (after the 4-byte
/// file header), so file position = 4 + offset.
fn block_address(offset: u32, size_class: u32) -> u32 {
    debug_assert!(size_class < 32, "size_class must fit in 5 bits");
    debug_assert!(
        offset.trailing_zeros() >= 5,
        "offset must be aligned to 32 bytes"
    );
    (offset & !0x1f) | size_class
}

// ---------------------------------------------------------------------------
// Plist helpers
// ---------------------------------------------------------------------------

/// Build the `bwsp` (window-state) binary plist blob.
pub(crate) fn build_bwsp_plist(window_width: u32, window_height: u32) -> Vec<u8> {
    let mut dict = BTreeMap::new();
    dict.insert(
        "ContainerShowSidebar".to_string(),
        PlistValue::Boolean(false),
    );
    dict.insert("ShowSidebar".to_string(), PlistValue::Boolean(false));
    dict.insert("ShowStatusBar".to_string(), PlistValue::Boolean(false));
    dict.insert("ShowTabView".to_string(), PlistValue::Boolean(false));
    dict.insert("ShowToolbar".to_string(), PlistValue::Boolean(false));
    dict.insert(
        "WindowBounds".to_string(),
        PlistValue::String(format!(
            "{{{{200, 120}}, {{{window_width}, {window_height}}}}}"
        )),
    );
    let val = PlistValue::Dictionary(dict.into_iter().collect());
    let mut buf = Vec::new();
    val.to_writer_binary(&mut buf)
        .expect("plist serialization should not fail for known-good data");
    buf
}

/// Build the `icvp` (icon-view preferences) binary plist blob.
pub(crate) fn build_icvp_plist(icon_size: u32, background_filename: &str, volume_name: &str) -> Vec<u8> {
    let mut dict: BTreeMap<String, PlistValue> = BTreeMap::new();
    dict.insert(
        "arrangeBy".to_string(),
        PlistValue::String("none".to_string()),
    );
    dict.insert("backgroundColorBlue".to_string(), PlistValue::Real(1.0));
    dict.insert("backgroundColorGreen".to_string(), PlistValue::Real(1.0));
    dict.insert("backgroundColorRed".to_string(), PlistValue::Real(1.0));
    dict.insert(
        "backgroundImageAlias".to_string(),
        PlistValue::Data(build_background_alias(background_filename, volume_name)),
    );
    dict.insert("backgroundType".to_string(), PlistValue::Integer(2.into()));
    dict.insert("gridOffsetX".to_string(), PlistValue::Real(0.0));
    dict.insert("gridOffsetY".to_string(), PlistValue::Real(0.0));
    dict.insert("gridSpacing".to_string(), PlistValue::Real(100.0));
    dict.insert(
        "iconSize".to_string(),
        PlistValue::Real(f64::from(icon_size)),
    );
    dict.insert("labelOnBottom".to_string(), PlistValue::Boolean(true));
    dict.insert("showIconPreview".to_string(), PlistValue::Boolean(true));
    dict.insert("showItemInfo".to_string(), PlistValue::Boolean(false));
    dict.insert("textSize".to_string(), PlistValue::Real(12.0));
    dict.insert(
        "viewOptionsVersion".to_string(),
        PlistValue::Integer(1.into()),
    );

    let val = PlistValue::Dictionary(dict.into_iter().collect());
    let mut buf = Vec::new();
    val.to_writer_binary(&mut buf)
        .expect("plist serialization should not fail for known-good data");
    buf
}

// ---------------------------------------------------------------------------
// Minimal macOS alias record
// ---------------------------------------------------------------------------

/// Build a minimal macOS Alias v2 record pointing to
/// `/.background/<filename>` on the given volume.
///
/// The Alias Manager format is documented in legacy Apple headers. We
/// construct the absolute minimum that Finder needs to resolve the
/// background image on a mounted DMG volume. The structure is:
///
///   - 4 bytes: user type (0)
///   - 2 bytes: record size (total alias length)
///   - 2 bytes: version (2)
///   - 2 bytes: kind (0 = file)
///   - 28 bytes: volume name (pascal string, padded to 28)
///   - 4 bytes: volume created date (0)
///   - 2 bytes: volume signature (`H+` = `0x482B` for HFS+)
///   - 2 bytes: volume type (0 = fixed)
///   - 4 bytes: parent directory ID (2 = root)
///   - 64 bytes: filename (pascal string, padded to 64)
///   - 4 bytes: file number (0)
///   - 4 bytes: file created date (0)
///   - 4 bytes: file type (0)
///   - 4 bytes: file creator (0)
///   - 2 bytes: nlvl from (0)
///   - 2 bytes: nlvl to (0)
///   - 4 bytes: volume attributes (0)
///   - 2 bytes: volume fs id (0)
///   - 10 bytes: reserved
///   - variable: tagged extra data
///
/// We then append tagged extra data entries for the full POSIX path so that
/// modern macOS can resolve the file.
/// Build a macOS Alias v2 record pointing to
/// `/.background/<filename>` on the given volume.
///
/// Tag format and numbering from Apple's Alias Manager (confirmed against
/// Finder-generated aliases via `create-dmg`):
///
/// | Tag | Encoding | Contents |
/// |-----|----------|----------|
/// | 0   | raw MacRoman/ASCII | parent directory name |
/// | 14  | 2-byte char count + UTF-16 BE | unicode filename |
/// | 15  | 2-byte char count + UTF-16 BE | unicode volume name |
/// | 18  | UTF-8 | POSIX path to target |
/// | 19  | UTF-8 | POSIX path to volume mount point |
/// | -1  | — | end sentinel |
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn build_background_alias(background_filename: &str, volume_name: &str) -> Vec<u8> {
    let posix_path = format!("/.background/{background_filename}");
    let volume_posix = format!("/Volumes/{volume_name}");

    // We build the alias body first, then prepend the 4-byte app type + size.
    let mut body = Vec::with_capacity(512);

    // version
    body.extend_from_slice(&2u16.to_be_bytes());
    // kind: 0 = file
    body.extend_from_slice(&0u16.to_be_bytes());

    // volume name: pascal string in 28 bytes
    let vol_bytes = volume_name.as_bytes();
    let vol_len = vol_bytes.len().min(27);
    body.push(vol_len as u8);
    body.extend_from_slice(&vol_bytes[..vol_len]);
    body.resize(body.len() + (27 - vol_len), 0);

    // volume created date (seconds since 1904-01-01, 0 = unknown)
    body.extend_from_slice(&0u32.to_be_bytes());
    // volume signature: "H+" for HFS+
    body.extend_from_slice(b"H+");
    // volume type: 5 = ejectable (matches Finder-generated DMG aliases)
    body.extend_from_slice(&5u16.to_be_bytes());
    // parent directory ID: 0 (unknown, resolved via tagged POSIX path)
    body.extend_from_slice(&0u32.to_be_bytes());

    // filename: pascal string in 64 bytes
    let fname_bytes = background_filename.as_bytes();
    let fname_len = fname_bytes.len().min(63);
    body.push(fname_len as u8);
    body.extend_from_slice(&fname_bytes[..fname_len]);
    body.resize(body.len() + (63 - fname_len), 0);

    // file number, created, type, creator
    body.extend_from_slice(&0u32.to_be_bytes()); // file number
    body.extend_from_slice(&0u32.to_be_bytes()); // created date
    body.extend_from_slice(&0u32.to_be_bytes()); // file type
    body.extend_from_slice(&0u32.to_be_bytes()); // file creator

    // nlvl from/to: 0xFFFF = unknown (matches Finder-generated aliases)
    body.extend_from_slice(&0xFFFFu16.to_be_bytes());
    body.extend_from_slice(&0xFFFFu16.to_be_bytes());

    // volume attributes
    body.extend_from_slice(&0u32.to_be_bytes());
    // volume fs id
    body.extend_from_slice(&0u16.to_be_bytes());
    // reserved (10 bytes)
    body.extend_from_slice(&[0u8; 10]);

    // --- Tagged extra data ---
    // Tag 0: parent directory name (raw MacRoman/ASCII)
    append_alias_tag_raw(&mut body, 0, ".background".as_bytes());
    // Tag 14: unicode filename (char count + UTF-16 BE)
    append_alias_tag_unicode(&mut body, 14, background_filename);
    // Tag 15: unicode volume name (char count + UTF-16 BE)
    append_alias_tag_unicode(&mut body, 15, volume_name);
    // Tag 18: POSIX path to target (UTF-8)
    append_alias_tag_raw(&mut body, 18, posix_path.as_bytes());
    // Tag 19: POSIX path to volume mount point (UTF-8)
    append_alias_tag_raw(&mut body, 19, volume_posix.as_bytes());

    // End-of-tags sentinel
    body.extend_from_slice(&(-1i16).to_be_bytes());
    body.extend_from_slice(&0u16.to_be_bytes());

    // Pad body to even length
    if body.len() % 2 != 0 {
        body.push(0);
    }

    // Build final alias: 4-byte app type (0) + 2-byte record size + body
    let record_size = (body.len() + 4 + 2) as u16;
    let mut alias = Vec::with_capacity(record_size as usize);
    alias.extend_from_slice(&0u32.to_be_bytes()); // app type
    alias.extend_from_slice(&record_size.to_be_bytes());
    alias.extend_from_slice(&body);
    alias
}

/// Append a raw-bytes alias tag (used for Mac Roman strings and UTF-8 paths).
#[allow(clippy::cast_possible_truncation)]
fn append_alias_tag_raw(buf: &mut Vec<u8>, tag: i16, data: &[u8]) {
    buf.extend_from_slice(&tag.to_be_bytes());
    buf.extend_from_slice(&(data.len() as u16).to_be_bytes());
    buf.extend_from_slice(data);
    // Pad to 2-byte alignment
    if data.len() % 2 != 0 {
        buf.push(0);
    }
}

/// Append a unicode alias tag (char count prefix + UTF-16 BE, used for tags 14/15).
#[allow(clippy::cast_possible_truncation)]
fn append_alias_tag_unicode(buf: &mut Vec<u8>, tag: i16, value: &str) {
    let utf16: Vec<u16> = value.encode_utf16().collect();
    let char_count = utf16.len() as u16;
    // Total data = 2-byte char count + UTF-16 bytes
    let byte_len = 2 + char_count * 2;
    buf.extend_from_slice(&tag.to_be_bytes());
    buf.extend_from_slice(&byte_len.to_be_bytes());
    buf.extend_from_slice(&char_count.to_be_bytes());
    for unit in &utf16 {
        buf.extend_from_slice(&unit.to_be_bytes());
    }
    // Pad to 2-byte alignment (byte_len is always even: 2 + even)
}

// ---------------------------------------------------------------------------
// macOS Bookmark (pBBk) builder
// ---------------------------------------------------------------------------

/// Build a macOS Bookmark (v0x10050000) pointing to
/// `/.background/<filename>` on the given DMG volume.
///
/// The bookmark format is little-endian throughout. Structure:
///   - 64-byte header (magic "book", size, version, header_size, security cookie)
///   - Payload: 4-byte first-TOC-offset + data items + TOC
///
/// Data items are: len(u32 LE) + type(u32 LE) + data + pad-to-4.
/// TOC: size(u32) + sentinel(u32) + id(u32) + next(u32) + count(u32) + entries.
/// Each TOC entry: key(u32) + data_offset(u32) + flags(u32).
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn build_background_bookmark(background_filename: &str, volume_name: &str) -> Vec<u8> {
    let volume_path = format!("/Volumes/{volume_name}");
    let volume_url = format!("file:///Volumes/{volume_name}/");

    // We build the payload (data items + TOC), then prepend the header.
    let mut payload = Vec::with_capacity(1024);

    // Reserve space for first-TOC-offset (filled in later).
    payload.extend_from_slice(&[0u8; 4]);

    // --- Data items ---
    // Track (item_offset_from_payload_start, key) pairs for the TOC.
    let mut toc_entries: Vec<(u32, u32)> = Vec::new();

    // Helper: append a data item, return its offset from payload start.
    fn append_item(payload: &mut Vec<u8>, item_type: u32, data: &[u8]) -> u32 {
        let offset = payload.len() as u32;
        payload.extend_from_slice(&(data.len() as u32).to_le_bytes());
        payload.extend_from_slice(&item_type.to_le_bytes());
        payload.extend_from_slice(data);
        // Pad to 4-byte alignment
        let pad = (4 - (data.len() % 4)) % 4;
        for _ in 0..pad {
            payload.push(0);
        }
        offset
    }

    fn append_string(payload: &mut Vec<u8>, s: &str) -> u32 {
        append_item(payload, 0x0101, s.as_bytes())
    }

    fn append_u32_item(payload: &mut Vec<u8>, v: u32) -> u32 {
        append_item(payload, 0x0303, &v.to_le_bytes())
    }

    fn append_array(payload: &mut Vec<u8>, offsets: &[u32]) -> u32 {
        let data: Vec<u8> = offsets.iter().flat_map(|o| o.to_le_bytes()).collect();
        append_item(payload, 0x0601, &data)
    }

    // CreationOptions (0xd010) = 0x20000200 (matches reference)
    let creation_opts_off = append_u32_item(&mut payload, 0x2000_0200);
    toc_entries.push((0xd010, creation_opts_off));

    // PathComponents (0x1004): ["Volumes", volume_name, ".background", filename]
    let pc_volumes = append_string(&mut payload, "Volumes");
    let pc_volname = append_string(&mut payload, volume_name);
    let pc_bgdir = append_string(&mut payload, ".background");
    let pc_filename = append_string(&mut payload, background_filename);
    let path_arr_off = append_array(&mut payload, &[pc_volumes, pc_volname, pc_bgdir, pc_filename]);
    toc_entries.push((0x1004, path_arr_off));

    // Dummy inode components (0x1005) — zeros, Finder falls back to path
    let inode_0 = append_item(&mut payload, 0x0304, &0u64.to_le_bytes());
    let inode_arr_off = append_array(&mut payload, &[inode_0, inode_0, inode_0, inode_0]);
    toc_entries.push((0x1005, inode_arr_off));

    // PropertyFlags (0x1010) — minimal flags from reference
    let prop_flags_data: [u8; 24] = [
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    let prop_off = append_item(&mut payload, 0x0201, &prop_flags_data);
    toc_entries.push((0x1010, prop_off));

    // CreationDate (0x1040) — zero (unknown)
    let cdate_off = append_item(&mut payload, 0x0400, &0.0f64.to_le_bytes());
    toc_entries.push((0x1040, cdate_off));

    // key 0x2000 — volume attribute array: [0xF000, 0, 1, 0, 0]
    let va_f000 = append_u32_item(&mut payload, 0xF000);
    let va_zero = append_u32_item(&mut payload, 0);
    let va_one = append_u32_item(&mut payload, 1);
    let va_arr_off = append_array(&mut payload, &[va_f000, va_zero, va_one, va_zero, va_zero]);
    toc_entries.push((0x2000, va_arr_off));

    // VolumePath (0x2002)
    let vol_path_off = append_string(&mut payload, &volume_path);
    toc_entries.push((0x2002, vol_path_off));

    // VolumeURL (0x2005)
    let vol_url_off = append_item(&mut payload, 0x0901, volume_url.as_bytes());
    toc_entries.push((0x2005, vol_url_off));

    // VolumeName (0x2010)
    let vol_name_off = append_string(&mut payload, volume_name);
    toc_entries.push((0x2010, vol_name_off));

    // VolumeUUID (0x2011) — generate a deterministic fake UUID
    let vol_uuid_off = append_string(&mut payload, "00000000-0000-0000-0000-000000000000");
    toc_entries.push((0x2011, vol_uuid_off));

    // VolumeCapacity (0x2012) — 50MB (typical small DMG)
    let vol_cap_off = append_item(&mut payload, 0x0304, &52428800u64.to_le_bytes());
    toc_entries.push((0x2012, vol_cap_off));

    // VolCreationDate (0x2013)
    let vol_cdate_off = append_item(&mut payload, 0x0400, &0.0f64.to_le_bytes());
    toc_entries.push((0x2013, vol_cdate_off));

    // VolPropertyFlags (0x2020) — from reference
    let vol_prop_data: [u8; 24] = [
        0x65, 0x02, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xef, 0x13, 0x00, 0x00, 0x01, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    let vol_prop_off = append_item(&mut payload, 0x0201, &vol_prop_data);
    toc_entries.push((0x2020, vol_prop_off));

    // key 0x2040 — from reference (value=4000)
    let v2040_off = append_u32_item(&mut payload, 4000);
    toc_entries.push((0x2040, v2040_off));

    // key 0xd001 — BoolTrue
    let bool_true_off = append_item(&mut payload, 0x0501, &[]);
    toc_entries.push((0xd001, bool_true_off));

    // --- Sort TOC entries by key (required for binary search) ---
    toc_entries.sort_by_key(|&(key, _)| key);

    // --- Build TOC ---
    let first_toc_offset = payload.len() as u32;

    // TOC: size + sentinel + id + next_toc + count + entries
    let toc_body_size = 4 * 4 + toc_entries.len() as u32 * 12;
    payload.extend_from_slice(&toc_body_size.to_le_bytes()); // total size of TOC body
    payload.extend_from_slice(&0xFFFF_FFFEu32.to_le_bytes()); // sentinel/record_type
    payload.extend_from_slice(&1u32.to_le_bytes()); // id
    payload.extend_from_slice(&0u32.to_le_bytes()); // next_toc (none)
    payload.extend_from_slice(&(toc_entries.len() as u32).to_le_bytes());

    for &(key, data_off) in &toc_entries {
        payload.extend_from_slice(&key.to_le_bytes());
        payload.extend_from_slice(&data_off.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes()); // flags
    }

    // Patch first-TOC-offset at the start of payload
    let toc_off_bytes = first_toc_offset.to_le_bytes();
    payload[0..4].copy_from_slice(&toc_off_bytes);

    // --- Build header ---
    let header_size: u32 = 64;
    let total_size = header_size + payload.len() as u32;

    let mut bookmark = Vec::with_capacity(total_size as usize);
    bookmark.extend_from_slice(b"book");
    bookmark.extend_from_slice(&total_size.to_le_bytes());
    bookmark.extend_from_slice(&0x1005_0000u32.to_le_bytes()); // version
    bookmark.extend_from_slice(&header_size.to_le_bytes());
    bookmark.extend_from_slice(&[0u8; 32]); // security cookie
    bookmark.extend_from_slice(b"0000000000"); // team id (10 bytes)
    bookmark.extend_from_slice(&[0u8; 6]); // reserved

    bookmark.extend_from_slice(&payload);
    bookmark
}

// ---------------------------------------------------------------------------
// Iloc helper
// ---------------------------------------------------------------------------

fn iloc_blob(x: u32, y: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(16);
    v.extend_from_slice(&x.to_be_bytes());
    v.extend_from_slice(&y.to_be_bytes());
    // Padding: 0xFFFFFFFF 0xFFFF0000 (observed in Finder-generated files)
    v.extend_from_slice(&0xFFFF_FFFFu32.to_be_bytes());
    v.extend_from_slice(&0xFFFF_0000u32.to_be_bytes());
    v
}

// ---------------------------------------------------------------------------
// DS_Store assembly
// ---------------------------------------------------------------------------

/// Build the complete records list, sorted by (filename, code).
///
/// Sort order uses UTF-16 code-unit comparison on the filename, then
/// byte comparison on the four-byte code.
fn build_records(layout: &DmgLayout) -> Vec<Record> {
    let bwsp_blob = build_bwsp_plist(layout.window_width, layout.window_height);
    let icvp_blob = build_icvp_plist(
        layout.icon_size,
        &layout.background_filename,
        &layout.volume_name,
    );

    let pbbk_blob = build_background_bookmark(
        &layout.background_filename,
        &layout.volume_name,
    );

    let mut records = vec![
        Record {
            filename: ".".to_string(),
            code: RecordCode::Bwsp,
            value: DsStoreValue::Blob(bwsp_blob),
        },
        Record {
            filename: ".".to_string(),
            code: RecordCode::Icvp,
            value: DsStoreValue::Blob(icvp_blob),
        },
        Record {
            filename: ".".to_string(),
            code: RecordCode::PBBk,
            value: DsStoreValue::Blob(pbbk_blob),
        },
        Record {
            filename: ".".to_string(),
            code: RecordCode::VSrn,
            value: DsStoreValue::Long(1),
        },
        Record {
            filename: "Applications".to_string(),
            code: RecordCode::Iloc,
            value: DsStoreValue::Blob(iloc_blob(layout.apps_link_x, layout.apps_link_y)),
        },
        Record {
            filename: layout.app_name.clone(),
            code: RecordCode::Iloc,
            value: DsStoreValue::Blob(iloc_blob(layout.app_x, layout.app_y)),
        },
    ];

    // Sort by (filename as UTF-16 code units, then code bytes).
    records.sort_by(|a, b| {
        let a_utf16: Vec<u16> = a.filename.encode_utf16().collect();
        let b_utf16: Vec<u16> = b.filename.encode_utf16().collect();
        a_utf16
            .cmp(&b_utf16)
            .then_with(|| a.code.as_bytes().cmp(&b.code.as_bytes()))
    });

    records
}

/// Serialize a B-tree leaf node containing the given records.
///
/// Leaf node layout:
///   - `pair_count`: 0 (u32 BE) -- signals this is a leaf
///   - `record_count` (u32 BE)
///   - serialized records
#[allow(clippy::cast_possible_truncation)]
fn serialize_leaf_node(records: &[Record]) -> Vec<u8> {
    let mut node = Vec::new();
    // pair_count = 0 -> leaf
    node.extend_from_slice(&0u32.to_be_bytes());
    // record count (always small; truncation safe)
    node.extend_from_slice(&(records.len() as u32).to_be_bytes());
    for rec in records {
        node.extend_from_slice(&rec.encode());
    }
    node
}

/// Serialize the DSDB root block.
///
/// Layout:
///   - `root_node` block ID (u32 BE)
///   - `num_internal_nodes` (u32 BE): 0 for a single-leaf tree
///   - `num_records` (u32 BE)
///   - `num_nodes` (u32 BE): 1
///   - `page_size` (u32 BE): `0x1000`
fn serialize_dsdb(root_node_block_id: u32, num_records: u32) -> Vec<u8> {
    let mut dsdb = Vec::with_capacity(20);
    dsdb.extend_from_slice(&root_node_block_id.to_be_bytes());
    dsdb.extend_from_slice(&0u32.to_be_bytes()); // num_internal_nodes
    dsdb.extend_from_slice(&num_records.to_be_bytes());
    dsdb.extend_from_slice(&1u32.to_be_bytes()); // num_nodes (1 leaf)
    dsdb.extend_from_slice(&0x0000_1000u32.to_be_bytes()); // page_size
    dsdb
}

/// Round `size` up to the next power of two, with a minimum of 32.
fn next_power_of_two(size: usize) -> usize {
    let min = 32;
    let v = size.max(min);
    v.next_power_of_two()
}

/// Return log2 of a power-of-two value.
fn log2(v: usize) -> u32 {
    debug_assert!(v.is_power_of_two());
    v.trailing_zeros()
}

/// Generate a complete `.DS_Store` file for a DMG installer layout.
///
/// The file uses a single B-tree leaf node with records for window
/// properties (`bwsp`, `icvp`, `vSrn`) and icon positions (`Iloc`).
#[allow(clippy::cast_possible_truncation)]
pub fn write_ds_store(layout: &DmgLayout) -> Vec<u8> {
    let records = build_records(layout);
    let num_records = records.len() as u32;

    // --- Serialize the two data blocks ---
    let dsdb_data = serialize_dsdb(0, num_records); // placeholder root; patched below
    let leaf_data = serialize_leaf_node(&records);

    // --- Compute block sizes (power-of-two, minimum 32) ---
    let dsdb_alloc = next_power_of_two(dsdb_data.len());
    let leaf_alloc = next_power_of_two(leaf_data.len());

    let dsdb_log2 = log2(dsdb_alloc);
    let leaf_log2 = log2(leaf_alloc);

    // --- Layout the data region ---
    //
    // Data region starts at file offset 4 (right after the 4-byte file header).
    // All data_offsets below are relative to byte 4.
    //
    // The allocator info block must be registered in its own offset table
    // as block 0 (matching what Finder/macOS expects). Layout:
    //
    //   data_offset 0..32         = Bud1 prelude (not a block)
    //   data_offset 32..32+dsdb   = DSDB block       → block[1]
    //   data_offset ...           = leaf node block   → block[2]
    //   data_offset ...           = allocator info    → block[0]
    //
    // The DSDB root_node points to block[2] (the leaf).
    // The DSDB TOC maps "DSDB" → block[1].

    let dsdb_offset: u32 = 32;
    let leaf_offset: u32 = dsdb_offset + dsdb_alloc as u32;
    let info_offset: u32 = leaf_offset + leaf_alloc as u32;

    let dsdb_addr = block_address(dsdb_offset, dsdb_log2);
    let leaf_addr = block_address(leaf_offset, leaf_log2);

    // DSDB root_node = block 2 (leaf), DSDB itself = block 1
    let dsdb_data = serialize_dsdb(2, num_records);

    // Allocator info needs to know its own address for block[0].
    // We compute info_alloc first, then build the block.
    let info_alloc = next_power_of_two(1200); // allocator info is ~1169 bytes; always 2048
    let info_log2 = log2(info_alloc);
    let info_addr = block_address(info_offset, info_log2);

    let info_block = build_allocator_info_v2(info_addr, dsdb_addr, leaf_addr);
    // Verify our size estimate was correct
    debug_assert!(
        info_block.len() <= info_alloc,
        "allocator info exceeds estimated alloc"
    );

    // --- Assemble the file ---
    let total_data_size = 32 + dsdb_alloc + leaf_alloc + info_alloc;
    let mut file = Vec::with_capacity(4 + total_data_size);

    // File header
    file.extend_from_slice(&[0, 0, 0, 1]);

    // Prelude (32 bytes in the data region):
    //   "Bud1" + info_offset + info_size + info_offset_copy + byte20 + 12 reserved
    file.extend_from_slice(b"Bud1");
    file.extend_from_slice(&info_offset.to_be_bytes());
    file.extend_from_slice(&(info_alloc as u32).to_be_bytes());
    file.extend_from_slice(&info_offset.to_be_bytes());
    // Byte 20: in Finder-generated files this contains the leaf block address.
    // Observed in the reference: 0x100c = leaf_addr. Purpose unknown but
    // Finder may use it as a quick-lookup for the root B-tree node.
    file.extend_from_slice(&leaf_addr.to_be_bytes());
    file.extend_from_slice(&[0u8; 12]);

    // DSDB block (padded to dsdb_alloc)
    file.extend_from_slice(&dsdb_data);
    file.resize(file.len() + (dsdb_alloc - dsdb_data.len()), 0);

    // Leaf node block (padded to leaf_alloc)
    file.extend_from_slice(&leaf_data);
    file.resize(file.len() + (leaf_alloc - leaf_data.len()), 0);

    // Allocator info block (padded to info_alloc)
    file.extend_from_slice(&info_block);
    file.resize(file.len() + (info_alloc - info_block.len()), 0);

    file
}

/// Build the allocator info block (v2: includes self-reference as block 0).
///
/// Block offset table layout (matching Finder-generated files):
///   - block[0] = allocator info block itself
///   - block[1] = DSDB root block
///   - block[2] = leaf node block
///
/// The DSDB TOC maps "DSDB" → block 1.
fn build_allocator_info_v2(info_addr: u32, dsdb_addr: u32, leaf_addr: u32) -> Vec<u8> {
    let mut info = Vec::with_capacity(2048);

    // Number of offsets (3: self, dsdb, leaf)
    let num_offsets: u32 = 3;
    info.extend_from_slice(&num_offsets.to_be_bytes());
    // 4 bytes reserved/zero
    info.extend_from_slice(&0u32.to_be_bytes());

    // Offsets array: [info_addr, dsdb_addr, leaf_addr] padded to 256 entries
    info.extend_from_slice(&info_addr.to_be_bytes());
    info.extend_from_slice(&dsdb_addr.to_be_bytes());
    info.extend_from_slice(&leaf_addr.to_be_bytes());
    // Pad remaining 253 entries with zeros
    for _ in 0..253 {
        info.extend_from_slice(&0u32.to_be_bytes());
    }

    // TOC: 1 entry → DSDB = block 1
    info.extend_from_slice(&1u32.to_be_bytes()); // count
    info.push(4); // key length
    info.extend_from_slice(b"DSDB"); // key
    info.extend_from_slice(&1u32.to_be_bytes()); // value = block ID 1

    // Free lists: 32 entries, each with count 0
    for _ in 0..32 {
        info.extend_from_slice(&0u32.to_be_bytes());
    }

    info
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Reader (test + debug support)
// ---------------------------------------------------------------------------

/// A parsed `.DS_Store` record (read back from bytes).
#[derive(Debug)]
pub struct ParsedRecord {
    pub filename: String,
    pub code: [u8; 4],
    pub type_code: [u8; 4],
    pub value: ParsedValue,
}

/// Parsed record value.
#[derive(Debug)]
pub enum ParsedValue {
    Blob(Vec<u8>),
    Long(i32),
    Bool(bool),
}

/// Parsed alias fixed header (first 150 bytes).
#[derive(Debug)]
pub struct ParsedAlias {
    pub record_size: u16,
    pub version: u16,
    pub kind: u16,
    pub volume_name: String,
    pub volume_created: u32,
    pub volume_sig: [u8; 2],
    pub volume_type: u16,
    pub parent_dir_id: u32,
    pub filename: String,
    pub file_number: u32,
    pub file_created: u32,
    pub nlvl_from: u16,
    pub nlvl_to: u16,
    pub vol_attrs: u32,
    pub tags: Vec<ParsedAliasTag>,
}

/// A single alias tag.
#[derive(Debug)]
pub struct ParsedAliasTag {
    pub tag: i16,
    pub data: Vec<u8>,
}

/// Read all records from a `.DS_Store` file.
pub fn read_ds_store(data: &[u8]) -> Vec<ParsedRecord> {
    assert!(data.len() >= 36, "too short for DS_Store");
    assert_eq!(&data[4..8], b"Bud1");

    let info_offset = u32::from_be_bytes(data[8..12].try_into().unwrap()) as usize;
    let info_pos = 4 + info_offset;

    let num_offsets = u32::from_be_bytes(data[info_pos..info_pos + 4].try_into().unwrap()) as usize;
    let offsets_start = info_pos + 8;

    let mut addrs = Vec::new();
    for i in 0..num_offsets {
        let addr = u32::from_be_bytes(
            data[offsets_start + i * 4..offsets_start + i * 4 + 4]
                .try_into()
                .unwrap(),
        );
        addrs.push(((addr & !0x1f) as usize, (addr & 0x1f) as usize));
    }

    // Find DSDB block via TOC
    let toc_pos = offsets_start + 256 * 4;
    let toc_count = u32::from_be_bytes(data[toc_pos..toc_pos + 4].try_into().unwrap()) as usize;
    let mut tp = toc_pos + 4;
    let mut dsdb_block = 0;
    for _ in 0..toc_count {
        let kl = data[tp] as usize;
        tp += 1;
        let _key = &data[tp..tp + kl];
        tp += kl;
        let val = u32::from_be_bytes(data[tp..tp + 4].try_into().unwrap()) as usize;
        tp += 4;
        dsdb_block = val;
    }

    let dsdb_pos = 4 + addrs[dsdb_block].0;
    let root_id = u32::from_be_bytes(data[dsdb_pos..dsdb_pos + 4].try_into().unwrap()) as usize;
    let leaf_pos = 4 + addrs[root_id].0;
    let record_count =
        u32::from_be_bytes(data[leaf_pos + 4..leaf_pos + 8].try_into().unwrap()) as usize;

    let mut pos = leaf_pos + 8;
    let mut records = Vec::new();
    for _ in 0..record_count {
        let name_len =
            u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        let name_utf16: Vec<u16> = (0..name_len)
            .map(|i| u16::from_be_bytes(data[pos + i * 2..pos + i * 2 + 2].try_into().unwrap()))
            .collect();
        let filename = String::from_utf16_lossy(&name_utf16);
        pos += name_len * 2;

        let mut code = [0u8; 4];
        code.copy_from_slice(&data[pos..pos + 4]);
        pos += 4;
        let mut type_code = [0u8; 4];
        type_code.copy_from_slice(&data[pos..pos + 4]);
        pos += 4;

        let value = match &type_code {
            b"blob" => {
                let bl =
                    u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
                pos += 4;
                let v = data[pos..pos + bl].to_vec();
                pos += bl;
                ParsedValue::Blob(v)
            }
            b"long" => {
                let v = i32::from_be_bytes(data[pos..pos + 4].try_into().unwrap());
                pos += 4;
                ParsedValue::Long(v)
            }
            b"bool" => {
                let v = data[pos] != 0;
                pos += 1;
                ParsedValue::Bool(v)
            }
            _ => panic!("unknown type code: {:?}", type_code),
        };

        records.push(ParsedRecord {
            filename,
            code,
            type_code,
            value,
        });
    }
    records
}

/// Parse alias bytes (as found in `icvp backgroundImageAlias`).
pub fn parse_alias(alias: &[u8]) -> ParsedAlias {
    let record_size = u16::from_be_bytes(alias[4..6].try_into().unwrap());
    let version = u16::from_be_bytes(alias[6..8].try_into().unwrap());
    let kind = u16::from_be_bytes(alias[8..10].try_into().unwrap());

    let vol_name_len = alias[10] as usize;
    let volume_name = String::from_utf8_lossy(&alias[11..11 + vol_name_len]).into_owned();

    let volume_created = u32::from_be_bytes(alias[38..42].try_into().unwrap());
    let mut volume_sig = [0u8; 2];
    volume_sig.copy_from_slice(&alias[42..44]);
    let volume_type = u16::from_be_bytes(alias[44..46].try_into().unwrap());
    let parent_dir_id = u32::from_be_bytes(alias[46..50].try_into().unwrap());

    let fname_len = alias[50] as usize;
    let filename = String::from_utf8_lossy(&alias[51..51 + fname_len]).into_owned();

    let file_number = u32::from_be_bytes(alias[114..118].try_into().unwrap());
    let file_created = u32::from_be_bytes(alias[118..122].try_into().unwrap());
    let nlvl_from = u16::from_be_bytes(alias[130..132].try_into().unwrap());
    let nlvl_to = u16::from_be_bytes(alias[132..134].try_into().unwrap());
    let vol_attrs = u32::from_be_bytes(alias[134..138].try_into().unwrap());

    // Parse tags starting at offset 150
    let mut tags = Vec::new();
    let mut tp = 150;
    while tp + 3 < alias.len() {
        let tag = i16::from_be_bytes(alias[tp..tp + 2].try_into().unwrap());
        if tag == -1 {
            break;
        }
        let tlen = u16::from_be_bytes(alias[tp + 2..tp + 4].try_into().unwrap()) as usize;
        tp += 4;
        let tdata = alias[tp..tp + tlen].to_vec();
        tp += tlen;
        if tlen % 2 != 0 {
            tp += 1; // alignment padding
        }
        tags.push(ParsedAliasTag { tag, data: tdata });
    }

    ParsedAlias {
        record_size,
        version,
        kind,
        volume_name,
        volume_created,
        volume_sig,
        volume_type,
        parent_dir_id,
        filename,
        file_number,
        file_created,
        nlvl_from,
        nlvl_to,
        vol_attrs,
        tags,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const REFERENCE: &[u8] = include_bytes!("../tests/fixtures/reference.DS_Store");

    fn test_layout() -> DmgLayout {
        DmgLayout {
            window_width: 660,
            window_height: 400,
            icon_size: 128,
            app_name: "JPEG Locker.app".to_string(),
            app_x: 160,
            app_y: 200,
            apps_link_x: 500,
            apps_link_y: 200,
            background_filename: DMG_BG_FILENAME.to_string(),
            volume_name: "JPEG Locker".to_string(),
        }
    }

    /// Write generated .DS_Store to target/ for manual inspection.
    fn write_debug_artifact(data: &[u8], name: &str) {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-artifacts");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join(name);
        std::fs::write(&path, data).unwrap();
        eprintln!("wrote {}: {} bytes", path.display(), data.len());
    }

    // -----------------------------------------------------------------------
    // Writer unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn output_starts_with_header_and_magic() {
        let data = write_ds_store(&test_layout());
        assert_eq!(&data[..4], &[0, 0, 0, 1], "file header");
        assert_eq!(&data[4..8], b"Bud1", "magic");
    }

    #[test]
    fn output_contains_record_codes() {
        let data = write_ds_store(&test_layout());
        assert!(contains_bytes(&data, b"Iloc"));
        assert!(contains_bytes(&data, b"bwsp"));
        assert!(contains_bytes(&data, b"icvp"));
        assert!(contains_bytes(&data, b"vSrn"));
    }

    #[test]
    fn output_contains_dsdb_toc_entry() {
        let data = write_ds_store(&test_layout());
        assert!(contains_bytes(&data, b"DSDB"));
    }

    #[test]
    fn prelude_offsets_match() {
        let data = write_ds_store(&test_layout());
        let offset1 = u32::from_be_bytes(data[8..12].try_into().unwrap());
        let offset2 = u32::from_be_bytes(data[16..20].try_into().unwrap());
        assert_eq!(offset1, offset2, "allocator info offset must appear twice");
    }

    /// Fix 1: The allocator info block must list itself as block[0].
    /// Without this self-reference, Finder silently ignores the entire .DS_Store.
    #[test]
    fn allocator_info_self_references_as_block_zero() {
        let data = write_ds_store(&test_layout());
        // Allocator info is at the offset stored in the prelude
        let info_offset = u32::from_be_bytes(data[8..12].try_into().unwrap());
        let info_pos = 4 + info_offset as usize; // +4 for file header

        // First u32 in allocator info = number of blocks
        let num_blocks = u32::from_be_bytes(data[info_pos..info_pos + 4].try_into().unwrap());
        assert_eq!(num_blocks, 3, "must have 3 blocks: info(0), dsdb(1), leaf(2)");

        // block[0] address should point back to the allocator info itself
        let block0_addr = u32::from_be_bytes(data[info_pos + 8..info_pos + 12].try_into().unwrap());
        let block0_offset = block0_addr & !0x1f;
        assert_eq!(block0_offset, info_offset, "block[0] must be the allocator info itself");
    }

    /// Fix 2: DSDB root_node must point to block[2] (the leaf), not block[1].
    /// With the wrong block index, Finder reads the DSDB as if it were the leaf
    /// and finds no records.
    #[test]
    fn dsdb_root_node_points_to_leaf_block() {
        let data = write_ds_store(&test_layout());
        // DSDB is block[1]. Read its address from the allocator info.
        let info_offset = u32::from_be_bytes(data[8..12].try_into().unwrap());
        let info_pos = 4 + info_offset as usize;
        let block1_addr = u32::from_be_bytes(data[info_pos + 12..info_pos + 16].try_into().unwrap());
        let dsdb_offset = (block1_addr & !0x1f) as usize;
        let dsdb_pos = 4 + dsdb_offset;

        let root_node = u32::from_be_bytes(data[dsdb_pos..dsdb_pos + 4].try_into().unwrap());
        assert_eq!(root_node, 2, "DSDB root_node must be block 2 (the leaf)");
    }

    /// Fix 3: Byte 20 in the Bud1 prelude must contain the leaf block address.
    /// When this is zero, Finder may skip the quick-path lookup and fail silently.
    #[test]
    fn prelude_byte20_contains_leaf_addr() {
        let data = write_ds_store(&test_layout());
        let byte20 = u32::from_be_bytes(data[20..24].try_into().unwrap());
        assert_ne!(byte20, 0, "byte 20 must not be zero");

        // It should match block[2] (leaf) from the allocator info
        let info_offset = u32::from_be_bytes(data[8..12].try_into().unwrap());
        let info_pos = 4 + info_offset as usize;
        let block2_addr = u32::from_be_bytes(data[info_pos + 16..info_pos + 20].try_into().unwrap());
        assert_eq!(byte20, block2_addr, "byte 20 must equal the leaf block address");
    }

    #[test]
    fn records_are_sorted_correctly() {
        let records = build_records(&test_layout());
        let filenames: Vec<&str> = records.iter().map(|r| r.filename.as_str()).collect();
        assert_eq!(
            filenames,
            [".", ".", ".", ".", "Applications", "JPEG Locker.app"]
        );
        assert_eq!(records[0].code, RecordCode::Bwsp);
        assert_eq!(records[1].code, RecordCode::Icvp);
        assert_eq!(records[2].code, RecordCode::PBBk);
        assert_eq!(records[3].code, RecordCode::VSrn);
    }

    #[test]
    fn iloc_blob_is_16_bytes() {
        let blob = iloc_blob(100, 200);
        assert_eq!(blob.len(), 16);
        assert_eq!(u32::from_be_bytes(blob[0..4].try_into().unwrap()), 100);
        assert_eq!(u32::from_be_bytes(blob[4..8].try_into().unwrap()), 200);
        assert_eq!(u32::from_be_bytes(blob[8..12].try_into().unwrap()), 0xFFFF_FFFF);
        assert_eq!(u32::from_be_bytes(blob[12..16].try_into().unwrap()), 0xFFFF_0000);
    }

    #[test]
    fn record_encoding_roundtrip() {
        let rec = Record {
            filename: ".".to_string(),
            code: RecordCode::VSrn,
            value: DsStoreValue::Long(1),
        };
        let encoded = rec.encode();
        assert_eq!(encoded.len(), 4 + 2 + 4 + 4 + 4);
        assert_eq!(u32::from_be_bytes(encoded[0..4].try_into().unwrap()), 1);
    }

    #[test]
    fn block_address_encoding() {
        let addr = block_address(32, 5);
        assert_eq!(addr & !0x1f, 32);
        assert_eq!(addr & 0x1f, 5);
    }

    #[test]
    fn leaf_node_starts_with_zero_pair_count() {
        let records = build_records(&test_layout());
        let node = serialize_leaf_node(&records);
        assert_eq!(u32::from_be_bytes(node[0..4].try_into().unwrap()), 0);
        assert_eq!(u32::from_be_bytes(node[4..8].try_into().unwrap()), 6);
    }

    #[test]
    fn dsdb_block_has_correct_structure() {
        let dsdb = serialize_dsdb(1, 5);
        assert_eq!(dsdb.len(), 20);
        assert_eq!(u32::from_be_bytes(dsdb[0..4].try_into().unwrap()), 1);
        assert_eq!(u32::from_be_bytes(dsdb[4..8].try_into().unwrap()), 0);
        assert_eq!(u32::from_be_bytes(dsdb[8..12].try_into().unwrap()), 5);
        assert_eq!(u32::from_be_bytes(dsdb[16..20].try_into().unwrap()), 0x1000);
    }

    #[test]
    fn bwsp_plist_is_valid_binary_plist() {
        let blob = build_bwsp_plist(660, 400);
        assert!(blob.starts_with(b"bplist"));
        let cursor = std::io::Cursor::new(&blob);
        let parsed = PlistValue::from_reader(cursor).expect("bwsp plist must parse");
        let dict = parsed.as_dictionary().expect("bwsp must be a dictionary");
        assert_eq!(dict.get("ShowToolbar").and_then(PlistValue::as_boolean), Some(false));
    }

    #[test]
    fn icvp_plist_is_valid_binary_plist() {
        let blob = build_icvp_plist(128, "background.png", "Test Volume");
        assert!(blob.starts_with(b"bplist"));
        let cursor = std::io::Cursor::new(&blob);
        let parsed = PlistValue::from_reader(cursor).expect("icvp plist must parse");
        let dict = parsed.as_dictionary().expect("icvp must be a dictionary");
        assert_eq!(dict.get("iconSize").and_then(PlistValue::as_real), Some(128.0));
        assert_eq!(
            dict.get("backgroundType").and_then(PlistValue::as_unsigned_integer),
            Some(2)
        );
        assert!(dict.get("backgroundImageAlias").is_some());
    }

    #[test]
    fn background_alias_starts_with_valid_header() {
        let alias = build_background_alias("background.png", "JPEG Locker");
        assert_eq!(u32::from_be_bytes(alias[0..4].try_into().unwrap()), 0);
        let size = u16::from_be_bytes(alias[4..6].try_into().unwrap());
        assert_eq!(size as usize, alias.len());
        assert_eq!(u16::from_be_bytes(alias[6..8].try_into().unwrap()), 2);
    }

    /// The .DS_Store always references the background as `bg.png` inside `.background/`.
    /// The alias POSIX path, bookmark path components, and alias filename must all
    /// use this canonical name so there's no mismatch with the file on disk.
    #[test]
    fn background_always_uses_canonical_filename() {
        let data = write_ds_store(&test_layout());
        let records = read_ds_store(&data);

        // Check icvp contains alias referencing bg.png
        let icvp = records.iter().find(|r| &r.code == b"icvp").unwrap();
        if let ParsedValue::Blob(ref blob) = icvp.value {
            let pl: plist::Dictionary = plist::from_bytes(blob).unwrap();
            let alias_data = pl
                .get("backgroundImageAlias")
                .and_then(PlistValue::as_data)
                .expect("icvp must have backgroundImageAlias");
            let alias = parse_alias(alias_data);
            assert_eq!(alias.filename, DMG_BG_FILENAME, "alias filename must be canonical");

            // Tag 18 = POSIX path, must end with /bg.png
            let posix_tag = alias.tags.iter().find(|t| t.tag == 18).unwrap();
            let posix = std::str::from_utf8(&posix_tag.data).unwrap();
            assert_eq!(posix, format!("/.background/{DMG_BG_FILENAME}"));
        } else {
            panic!("icvp must be a blob");
        }

        // Check pBBk bookmark contains bg.png in its path components
        let pbbk = records.iter().find(|r| &r.code == b"pBBk").unwrap();
        if let ParsedValue::Blob(ref blob) = pbbk.value {
            // The bookmark embeds UTF-8 path components; bg.png must appear
            let has_filename = blob
                .windows(DMG_BG_FILENAME.len())
                .any(|w| w == DMG_BG_FILENAME.as_bytes());
            assert!(has_filename, "bookmark must contain canonical bg filename");
        } else {
            panic!("pBBk must be a blob");
        }
    }

    #[test]
    fn next_power_of_two_basics() {
        assert_eq!(next_power_of_two(1), 32);
        assert_eq!(next_power_of_two(20), 32);
        assert_eq!(next_power_of_two(32), 32);
        assert_eq!(next_power_of_two(33), 64);
        assert_eq!(next_power_of_two(1000), 1024);
    }

    // -----------------------------------------------------------------------
    // Reader roundtrip tests
    // -----------------------------------------------------------------------

    #[test]
    fn roundtrip_read_our_output() {
        let data = write_ds_store(&test_layout());
        write_debug_artifact(&data, "ours.DS_Store");

        let records = read_ds_store(&data);
        assert_eq!(records.len(), 6);

        // Check filenames and codes
        assert_eq!(records[0].filename, ".");
        assert_eq!(&records[0].code, b"bwsp");
        assert_eq!(records[1].filename, ".");
        assert_eq!(&records[1].code, b"icvp");
        assert_eq!(records[2].filename, ".");
        assert_eq!(&records[2].code, b"pBBk");
        assert_eq!(records[3].filename, ".");
        assert_eq!(&records[3].code, b"vSrn");
        assert_eq!(records[4].filename, "Applications");
        assert_eq!(&records[4].code, b"Iloc");
        assert_eq!(records[5].filename, "JPEG Locker.app");
        assert_eq!(&records[5].code, b"Iloc");

        // Check Iloc positions
        if let ParsedValue::Blob(ref b) = records[4].value {
            assert_eq!(u32::from_be_bytes(b[0..4].try_into().unwrap()), 500);
            assert_eq!(u32::from_be_bytes(b[4..8].try_into().unwrap()), 200);
        } else {
            panic!("Applications Iloc should be a blob");
        }
        if let ParsedValue::Blob(ref b) = records[5].value {
            assert_eq!(u32::from_be_bytes(b[0..4].try_into().unwrap()), 160);
            assert_eq!(u32::from_be_bytes(b[4..8].try_into().unwrap()), 200);
        } else {
            panic!("app Iloc should be a blob");
        }
    }

    #[test]
    fn roundtrip_read_our_alias() {
        let alias_bytes = build_background_alias(DMG_BG_FILENAME, "JPEG Locker");
        let alias = parse_alias(&alias_bytes);

        assert_eq!(alias.version, 2);
        assert_eq!(alias.kind, 0);
        assert_eq!(alias.volume_name, "JPEG Locker");
        assert_eq!(&alias.volume_sig, b"H+");
        assert_eq!(alias.volume_type, 5);
        assert_eq!(alias.filename, DMG_BG_FILENAME);
        assert_eq!(alias.nlvl_from, 0xFFFF);
        assert_eq!(alias.nlvl_to, 0xFFFF);

        // Check tags
        let tag_ids: Vec<i16> = alias.tags.iter().map(|t| t.tag).collect();
        assert_eq!(tag_ids, [0, 14, 15, 18, 19]);

        // Tag 0: parent dir name (raw ASCII)
        assert_eq!(alias.tags[0].data, b".background");

        // Tag 14: unicode filename (char count + UTF-16)
        let nchars = u16::from_be_bytes(alias.tags[1].data[0..2].try_into().unwrap());
        let bg_chars = DMG_BG_FILENAME.chars().count() as u16;
        assert_eq!(nchars, bg_chars);
        let utf16: Vec<u16> = (0..nchars as usize)
            .map(|i| {
                u16::from_be_bytes(
                    alias.tags[1].data[2 + i * 2..4 + i * 2].try_into().unwrap(),
                )
            })
            .collect();
        assert_eq!(String::from_utf16_lossy(&utf16), DMG_BG_FILENAME);

        // Tag 15: unicode volume name
        let nchars = u16::from_be_bytes(alias.tags[2].data[0..2].try_into().unwrap());
        assert_eq!(nchars, 11);

        // Tag 18: POSIX path
        assert_eq!(
            std::str::from_utf8(&alias.tags[3].data).unwrap(),
            format!("/.background/{DMG_BG_FILENAME}")
        );

        // Tag 19: volume POSIX path
        assert_eq!(
            std::str::from_utf8(&alias.tags[4].data).unwrap(),
            "/Volumes/JPEG Locker"
        );
    }

    // -----------------------------------------------------------------------
    // Reference fixture comparison tests
    // -----------------------------------------------------------------------

    #[test]
    fn reference_is_readable() {
        let records = read_ds_store(REFERENCE);
        write_debug_artifact(REFERENCE, "reference.DS_Store");

        let codes: Vec<&[u8; 4]> = records.iter().map(|r| &r.code).collect();
        assert!(codes.contains(&&*b"bwsp"));
        assert!(codes.contains(&&*b"icvp"));
        assert!(codes.contains(&&*b"vSrn"));
        assert!(codes.contains(&&*b"Iloc"));
    }

    #[test]
    fn roundtrip_our_bookmark() {
        let bm = build_background_bookmark(DMG_BG_FILENAME, "JPEG Locker");
        assert_eq!(&bm[..4], b"book", "bookmark magic");
        let total_size = u32::from_le_bytes(bm[4..8].try_into().unwrap()) as usize;
        assert_eq!(total_size, bm.len(), "total size must match actual length");
        let version = u32::from_le_bytes(bm[8..12].try_into().unwrap());
        assert_eq!(version, 0x1005_0000, "version");
        let header_size = u32::from_le_bytes(bm[12..16].try_into().unwrap()) as usize;
        assert_eq!(header_size, 64);

        // The first TOC offset (relative to payload) must be within bounds
        let payload_start = header_size;
        let first_toc_rel =
            u32::from_le_bytes(bm[payload_start..payload_start + 4].try_into().unwrap()) as usize;
        let first_toc_abs = payload_start + first_toc_rel;
        assert!(first_toc_abs < bm.len(), "TOC offset must be in bounds");

        // TOC sentinel
        let sentinel =
            u32::from_le_bytes(bm[first_toc_abs + 4..first_toc_abs + 8].try_into().unwrap());
        assert_eq!(sentinel, 0xFFFF_FFFE, "TOC sentinel");

        write_debug_artifact(&bm, "ours_pBBk.bin");
    }

    #[test]
    fn compare_iloc_positions_with_reference() {
        let ref_records = read_ds_store(REFERENCE);
        let our_data = write_ds_store(&test_layout());
        let our_records = read_ds_store(&our_data);

        for name in &["Applications", "JPEG Locker.app"] {
            let ref_iloc = ref_records
                .iter()
                .find(|r| r.filename == *name && &r.code == b"Iloc")
                .expect("reference should have Iloc");
            let our_iloc = our_records
                .iter()
                .find(|r| r.filename == *name && &r.code == b"Iloc")
                .expect("ours should have Iloc");

            if let (ParsedValue::Blob(ref rb), ParsedValue::Blob(ref ob)) =
                (&ref_iloc.value, &our_iloc.value)
            {
                let ref_x = u32::from_be_bytes(rb[0..4].try_into().unwrap());
                let ref_y = u32::from_be_bytes(rb[4..8].try_into().unwrap());
                let our_x = u32::from_be_bytes(ob[0..4].try_into().unwrap());
                let our_y = u32::from_be_bytes(ob[4..8].try_into().unwrap());
                assert_eq!((our_x, our_y), (ref_x, ref_y), "Iloc mismatch for {name}");
                // Check padding matches
                assert_eq!(&ob[8..], &rb[8..], "Iloc padding mismatch for {name}");
            }
        }
    }

    #[test]
    fn compare_icvp_fields_with_reference() {
        let ref_records = read_ds_store(REFERENCE);
        let our_data = write_ds_store(&test_layout());
        let our_records = read_ds_store(&our_data);

        let ref_icvp = ref_records.iter().find(|r| &r.code == b"icvp").unwrap();
        let our_icvp = our_records.iter().find(|r| &r.code == b"icvp").unwrap();

        let ref_blob = match &ref_icvp.value {
            ParsedValue::Blob(b) => b,
            _ => panic!("icvp should be blob"),
        };
        let our_blob = match &our_icvp.value {
            ParsedValue::Blob(b) => b,
            _ => panic!("icvp should be blob"),
        };

        let ref_pl: plist::Dictionary = plist::from_bytes(ref_blob).unwrap();
        let our_pl: plist::Dictionary = plist::from_bytes(our_blob).unwrap();

        // Fields that must match
        assert_eq!(
            our_pl.get("backgroundType"),
            ref_pl.get("backgroundType"),
            "backgroundType mismatch"
        );
        assert_eq!(
            our_pl.get("iconSize"),
            ref_pl.get("iconSize"),
            "iconSize mismatch"
        );
        assert_eq!(
            our_pl.get("arrangeBy"),
            ref_pl.get("arrangeBy"),
            "arrangeBy mismatch"
        );
        assert_eq!(
            our_pl.get("showIconPreview"),
            ref_pl.get("showIconPreview"),
        );
        assert_eq!(
            our_pl.get("viewOptionsVersion"),
            ref_pl.get("viewOptionsVersion"),
        );

        // Both must have backgroundImageAlias
        assert!(our_pl.get("backgroundImageAlias").is_some());
        assert!(ref_pl.get("backgroundImageAlias").is_some());
    }

    #[test]
    fn compare_alias_structure_with_reference() {
        let ref_records = read_ds_store(REFERENCE);
        let our_data = write_ds_store(&test_layout());
        let our_records = read_ds_store(&our_data);

        let ref_icvp_blob = match &ref_records.iter().find(|r| &r.code == b"icvp").unwrap().value {
            ParsedValue::Blob(b) => b.clone(),
            _ => panic!(),
        };
        let our_icvp_blob = match &our_records.iter().find(|r| &r.code == b"icvp").unwrap().value {
            ParsedValue::Blob(b) => b.clone(),
            _ => panic!(),
        };

        let ref_pl: plist::Dictionary = plist::from_bytes(&ref_icvp_blob).unwrap();
        let our_pl: plist::Dictionary = plist::from_bytes(&our_icvp_blob).unwrap();

        let ref_alias_bytes = ref_pl
            .get("backgroundImageAlias")
            .unwrap()
            .as_data()
            .unwrap();
        let our_alias_bytes = our_pl
            .get("backgroundImageAlias")
            .unwrap()
            .as_data()
            .unwrap();

        let ref_alias = parse_alias(ref_alias_bytes);
        let our_alias = parse_alias(our_alias_bytes);

        eprintln!("--- Reference alias ---");
        eprintln!("{ref_alias:#?}");
        eprintln!("--- Our alias ---");
        eprintln!("{our_alias:#?}");

        // Structural fields that must match (filename differs: we use bg.png,
        // reference used dmg-background-2.png — that's intentional)
        assert_eq!(our_alias.version, ref_alias.version, "alias version");
        assert_eq!(our_alias.kind, ref_alias.kind, "alias kind");
        assert_eq!(our_alias.volume_name, ref_alias.volume_name, "volume name");
        assert_eq!(our_alias.volume_sig, ref_alias.volume_sig, "volume sig");
        assert_eq!(our_alias.volume_type, ref_alias.volume_type, "volume type");
        assert_eq!(our_alias.filename, DMG_BG_FILENAME, "our filename must be canonical");
        assert_eq!(our_alias.nlvl_from, ref_alias.nlvl_from, "nlvl_from");
        assert_eq!(our_alias.nlvl_to, ref_alias.nlvl_to, "nlvl_to");

        // Tags we write must be present in reference too
        let our_tag_ids: Vec<i16> = our_alias.tags.iter().map(|t| t.tag).collect();
        let ref_tag_ids: Vec<i16> = ref_alias.tags.iter().map(|t| t.tag).collect();
        for tag_id in &our_tag_ids {
            assert!(
                ref_tag_ids.contains(tag_id),
                "our tag {tag_id} not found in reference (ref has {ref_tag_ids:?})"
            );
        }

        // Compare tags that don't contain the filename (tag 0 = parent dir, tag 15 = volume name).
        // Tags 14 (unicode filename), 18 (POSIX path), 19 (volume path) differ by design.
        for tag_id in &[0i16, 15] {
            let our_tag = our_alias.tags.iter().find(|t| t.tag == *tag_id);
            let ref_tag = ref_alias.tags.iter().find(|t| t.tag == *tag_id);
            if let (Some(ours), Some(refs)) = (our_tag, ref_tag) {
                assert_eq!(ours.data, refs.data, "tag {tag_id} data mismatch");
            }
        }
    }

    /// Scan `haystack` for the byte sequence `needle`.
    fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }
}
