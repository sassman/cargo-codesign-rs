//! Alias V2 binary format — encode and decode.
//!
//! Wire format (big-endian):
//!   6-byte prefix: app_type(4 BE, =0) + record_size(2 BE)
//!   144-byte fixed body starting at offset 6:
//!     version(u16=2), kind(u16), vol_name(28-byte pascal), vol_created(u32),
//!     vol_sig(2), vol_type(u16), parent_dir_id(u32), filename(64-byte pascal),
//!     file_number(u32), file_created(u32), file_type(4), file_creator(4),
//!     nlvl_from(u16), nlvl_to(u16), vol_attrs(u32), vol_fs_id(u16), reserved(10)
//!   Variable-length tags until sentinel (-1, 0x0000)
//!   Padded to even length

use super::types::{BinaryDecode, BinaryEncode, DecodeError};

/// Prefix: 4-byte app_type + 2-byte record_size.
const PREFIX_LEN: usize = 6;
/// Fixed body: version through reserved (144 bytes).
const FIXED_BODY_LEN: usize = 144;
/// Tags start at this offset from the beginning of the alias.
const TAGS_OFFSET: usize = PREFIX_LEN + FIXED_BODY_LEN;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AliasV2 {
    pub(crate) kind: AliasKind,
    pub(crate) volume_name: String,
    pub(crate) volume_created: u32,
    pub(crate) volume_signature: [u8; 2],
    pub(crate) volume_type: u16,
    pub(crate) parent_dir_id: u32,
    pub(crate) filename: String,
    pub(crate) file_number: u32,
    pub(crate) file_created: u32,
    pub(crate) file_type: [u8; 4],
    pub(crate) file_creator: [u8; 4],
    pub(crate) nlvl_from: u16,
    pub(crate) nlvl_to: u16,
    pub(crate) vol_attrs: u32,
    pub(crate) vol_fs_id: u16,
    pub(crate) tags: Vec<AliasTag>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AliasKind {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AliasTag {
    ParentDirName(String),
    UnicodeFilename(String),
    UnicodeVolumeName(String),
    PosixPath(String),
    VolumeMountPoint(String),
    Unknown { tag: u16, data: Vec<u8> },
}

// ---------------------------------------------------------------------------
// Encode
// ---------------------------------------------------------------------------

impl BinaryEncode for AliasV2 {
    #[allow(clippy::cast_possible_truncation)]
    fn encode(&self) -> Vec<u8> {
        // Build the body (everything after the 6-byte prefix).
        let mut body = Vec::with_capacity(512);

        // version = 2
        body.extend_from_slice(&2u16.to_be_bytes());
        // kind
        body.extend_from_slice(&self.kind.to_u16().to_be_bytes());

        // volume name: pascal string in 28 bytes (1 len + up to 27 chars)
        write_pascal_string(&mut body, &self.volume_name, 28);

        // volume created (Mac epoch seconds)
        body.extend_from_slice(&self.volume_created.to_be_bytes());
        // volume signature
        body.extend_from_slice(&self.volume_signature);
        // volume type
        body.extend_from_slice(&self.volume_type.to_be_bytes());
        // parent directory ID
        body.extend_from_slice(&self.parent_dir_id.to_be_bytes());

        // filename: pascal string in 64 bytes (1 len + up to 63 chars)
        write_pascal_string(&mut body, &self.filename, 64);

        // file number, created, type, creator
        body.extend_from_slice(&self.file_number.to_be_bytes());
        body.extend_from_slice(&self.file_created.to_be_bytes());
        body.extend_from_slice(&self.file_type);
        body.extend_from_slice(&self.file_creator);

        // nlvl from / to
        body.extend_from_slice(&self.nlvl_from.to_be_bytes());
        body.extend_from_slice(&self.nlvl_to.to_be_bytes());

        // volume attributes
        body.extend_from_slice(&self.vol_attrs.to_be_bytes());
        // volume FS ID
        body.extend_from_slice(&self.vol_fs_id.to_be_bytes());
        // reserved (10 bytes)
        body.extend_from_slice(&[0u8; 10]);

        debug_assert_eq!(body.len(), FIXED_BODY_LEN);

        // Tags
        for tag in &self.tags {
            encode_tag(&mut body, tag);
        }

        // End-of-tags sentinel
        body.extend_from_slice(&(-1i16).to_be_bytes());
        body.extend_from_slice(&0u16.to_be_bytes());

        // Pad body to even length
        if body.len() % 2 != 0 {
            body.push(0);
        }

        // Build final: 4-byte app type (0) + 2-byte record_size + body
        let record_size = (body.len() + PREFIX_LEN) as u16;
        let mut alias = Vec::with_capacity(record_size as usize);
        alias.extend_from_slice(&0u32.to_be_bytes()); // app type
        alias.extend_from_slice(&record_size.to_be_bytes());
        alias.extend_from_slice(&body);
        alias
    }
}

/// Write a pascal string into exactly `total_bytes` (1 length byte + data + zero padding).
#[allow(clippy::cast_possible_truncation)]
fn write_pascal_string(buf: &mut Vec<u8>, s: &str, total_bytes: usize) {
    let bytes = s.as_bytes();
    let max_data = total_bytes - 1;
    let len = bytes.len().min(max_data);
    buf.push(len as u8);
    buf.extend_from_slice(&bytes[..len]);
    // Zero-pad the remainder
    let padding = max_data - len;
    buf.extend(std::iter::repeat_n(0u8, padding));
}

/// Encode a single alias tag into the buffer.
#[allow(clippy::cast_possible_truncation)]
fn encode_tag(buf: &mut Vec<u8>, tag: &AliasTag) {
    match tag {
        AliasTag::ParentDirName(s) => append_tag_raw(buf, 0, s.as_bytes()),
        AliasTag::UnicodeFilename(s) => append_tag_unicode(buf, 14, s),
        AliasTag::UnicodeVolumeName(s) => append_tag_unicode(buf, 15, s),
        AliasTag::PosixPath(s) => append_tag_raw(buf, 18, s.as_bytes()),
        AliasTag::VolumeMountPoint(s) => append_tag_raw(buf, 19, s.as_bytes()),
        AliasTag::Unknown { tag: t, data } => append_tag_raw(buf, *t as i16, data),
    }
}

/// Append a raw-bytes alias tag (Mac Roman strings and UTF-8 paths).
#[allow(clippy::cast_possible_truncation)]
fn append_tag_raw(buf: &mut Vec<u8>, tag: i16, data: &[u8]) {
    buf.extend_from_slice(&tag.to_be_bytes());
    buf.extend_from_slice(&(data.len() as u16).to_be_bytes());
    buf.extend_from_slice(data);
    if data.len() % 2 != 0 {
        buf.push(0);
    }
}

/// Append a unicode alias tag (char count prefix + UTF-16 BE, for tags 14/15).
#[allow(clippy::cast_possible_truncation)]
fn append_tag_unicode(buf: &mut Vec<u8>, tag: i16, value: &str) {
    let utf16: Vec<u16> = value.encode_utf16().collect();
    let char_count = utf16.len() as u16;
    // Total data = 2-byte char count + UTF-16 code units
    let byte_len = 2 + char_count * 2;
    buf.extend_from_slice(&tag.to_be_bytes());
    buf.extend_from_slice(&byte_len.to_be_bytes());
    buf.extend_from_slice(&char_count.to_be_bytes());
    for unit in &utf16 {
        buf.extend_from_slice(&unit.to_be_bytes());
    }
    // byte_len is always even (2 + 2*n), so no padding needed.
}

// ---------------------------------------------------------------------------
// Decode
// ---------------------------------------------------------------------------

impl BinaryDecode for AliasV2 {
    #[allow(clippy::cast_possible_truncation)]
    fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < TAGS_OFFSET {
            return Err(DecodeError::TooShort {
                expected: TAGS_OFFSET,
                got: data.len(),
            });
        }

        // Skip app_type (4 bytes) and record_size (2 bytes).
        // Body starts at offset 6.
        let kind_raw = u16::from_be_bytes(read2(data, 8)?);
        let kind = AliasKind::from_u16(kind_raw)?;

        let vol_name_len = data[10] as usize;
        let vol_name_end = 11 + vol_name_len;
        if vol_name_end > 38 {
            return Err(DecodeError::Other(format!(
                "volume name length {vol_name_len} exceeds 27-byte pascal field"
            )));
        }
        let volume_name = String::from_utf8_lossy(&data[11..vol_name_end]).into_owned();

        let volume_created = u32::from_be_bytes(read4(data, 38)?);
        let mut volume_signature = [0u8; 2];
        volume_signature.copy_from_slice(&data[42..44]);
        let volume_type = u16::from_be_bytes(read2(data, 44)?);
        let parent_dir_id = u32::from_be_bytes(read4(data, 46)?);

        let fname_len = data[50] as usize;
        let fname_end = 51 + fname_len;
        if fname_end > 114 {
            return Err(DecodeError::Other(format!(
                "filename length {fname_len} exceeds 63-byte pascal field"
            )));
        }
        let filename = String::from_utf8_lossy(&data[51..fname_end]).into_owned();

        let file_number = u32::from_be_bytes(read4(data, 114)?);
        let file_created = u32::from_be_bytes(read4(data, 118)?);

        let mut file_type = [0u8; 4];
        file_type.copy_from_slice(&data[122..126]);
        let mut file_creator = [0u8; 4];
        file_creator.copy_from_slice(&data[126..130]);

        let nlvl_from = u16::from_be_bytes(read2(data, 130)?);
        let nlvl_to = u16::from_be_bytes(read2(data, 132)?);
        let vol_attrs = u32::from_be_bytes(read4(data, 134)?);
        let vol_fs_id = u16::from_be_bytes(read2(data, 138)?);

        // Parse tags starting at offset 150
        let mut tags = Vec::new();
        let mut pos = TAGS_OFFSET;
        while pos + 3 < data.len() {
            let tag_num = i16::from_be_bytes(read2(data, pos)?);
            if tag_num == -1 {
                break;
            }
            let tlen = u16::from_be_bytes(read2(data, pos + 2)?) as usize;
            pos += 4;
            if pos + tlen > data.len() {
                return Err(DecodeError::TooShort {
                    expected: pos + tlen,
                    got: data.len(),
                });
            }
            let tdata = &data[pos..pos + tlen];
            pos += tlen;
            if tlen % 2 != 0 {
                pos += 1; // alignment padding
            }
            tags.push(decode_tag(tag_num, tdata)?);
        }

        Ok(AliasV2 {
            kind,
            volume_name,
            volume_created,
            volume_signature,
            volume_type,
            parent_dir_id,
            filename,
            file_number,
            file_created,
            file_type,
            file_creator,
            nlvl_from,
            nlvl_to,
            vol_attrs,
            vol_fs_id,
            tags,
        })
    }
}

/// Decode a single alias tag from its raw data.
fn decode_tag(tag_num: i16, data: &[u8]) -> Result<AliasTag, DecodeError> {
    match tag_num {
        0 => Ok(AliasTag::ParentDirName(
            String::from_utf8_lossy(data).into_owned(),
        )),
        14 => Ok(AliasTag::UnicodeFilename(decode_unicode_payload(data)?)),
        15 => Ok(AliasTag::UnicodeVolumeName(decode_unicode_payload(data)?)),
        18 => Ok(AliasTag::PosixPath(
            String::from_utf8_lossy(data).into_owned(),
        )),
        19 => Ok(AliasTag::VolumeMountPoint(
            String::from_utf8_lossy(data).into_owned(),
        )),
        _ => Ok(AliasTag::Unknown {
            tag: tag_num as u16,
            data: data.to_vec(),
        }),
    }
}

/// Decode a unicode tag payload: 2-byte char count + UTF-16 BE code units.
fn decode_unicode_payload(data: &[u8]) -> Result<String, DecodeError> {
    if data.len() < 2 {
        return Err(DecodeError::TooShort {
            expected: 2,
            got: data.len(),
        });
    }
    let char_count = u16::from_be_bytes([data[0], data[1]]) as usize;
    let expected = 2 + char_count * 2;
    if data.len() < expected {
        return Err(DecodeError::TooShort {
            expected,
            got: data.len(),
        });
    }
    let mut units = Vec::with_capacity(char_count);
    for i in 0..char_count {
        let offset = 2 + i * 2;
        units.push(u16::from_be_bytes([data[offset], data[offset + 1]]));
    }
    String::from_utf16(&units).map_err(|e| DecodeError::Other(format!("invalid UTF-16: {e}")))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

impl AliasKind {
    fn to_u16(&self) -> u16 {
        match self {
            Self::File => 0,
            Self::Directory => 1,
        }
    }

    fn from_u16(v: u16) -> Result<Self, DecodeError> {
        match v {
            0 => Ok(Self::File),
            1 => Ok(Self::Directory),
            _ => Err(DecodeError::Other(format!("unknown alias kind: {v}"))),
        }
    }
}

/// Read 2 bytes at the given offset, returning an array for `from_be_bytes`.
fn read2(data: &[u8], offset: usize) -> Result<[u8; 2], DecodeError> {
    data.get(offset..offset + 2)
        .and_then(|s| s.try_into().ok())
        .ok_or(DecodeError::TooShort {
            expected: offset + 2,
            got: data.len(),
        })
}

/// Read 4 bytes at the given offset, returning an array for `from_be_bytes`.
fn read4(data: &[u8], offset: usize) -> Result<[u8; 4], DecodeError> {
    data.get(offset..offset + 4)
        .and_then(|s| s.try_into().ok())
        .ok_or(DecodeError::TooShort {
            expected: offset + 4,
            got: data.len(),
        })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_alias() -> AliasV2 {
        AliasV2 {
            kind: AliasKind::File,
            volume_name: "JPEG Locker".to_string(),
            volume_created: 0,
            volume_signature: *b"H+",
            volume_type: 5,
            parent_dir_id: 0,
            filename: "bg.png".to_string(),
            file_number: 0,
            file_created: 0,
            file_type: [0; 4],
            file_creator: [0; 4],
            nlvl_from: 0xFFFF,
            nlvl_to: 0xFFFF,
            vol_attrs: 0,
            vol_fs_id: 0,
            tags: vec![
                AliasTag::ParentDirName(".background".to_string()),
                AliasTag::UnicodeFilename("bg.png".to_string()),
                AliasTag::UnicodeVolumeName("JPEG Locker".to_string()),
                AliasTag::PosixPath("/.background/bg.png".to_string()),
                AliasTag::VolumeMountPoint("/Volumes/JPEG Locker".to_string()),
            ],
        }
    }

    #[test]
    fn encode_starts_with_valid_header() {
        let encoded = test_alias().encode();

        // app_type = 0
        assert_eq!(&encoded[0..4], &0u32.to_be_bytes());
        // record_size is a u16 at bytes 4..6
        let record_size = u16::from_be_bytes(encoded[4..6].try_into().unwrap());
        assert_eq!(record_size as usize, encoded.len());
        // version = 2 at bytes 6..8
        let version = u16::from_be_bytes(encoded[6..8].try_into().unwrap());
        assert_eq!(version, 2);
        // kind = 0 (File) at bytes 8..10
        let kind = u16::from_be_bytes(encoded[8..10].try_into().unwrap());
        assert_eq!(kind, 0);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let original = test_alias();
        let encoded = original.encode();
        let decoded = AliasV2::decode(&encoded).expect("decode should succeed");
        assert_eq!(original, decoded);
    }

    #[test]
    fn byte_identical_to_old_alias_builder() {
        let old_bytes =
            crate::ds_store_old::build_background_alias("bg.png", "JPEG Locker");
        let new_bytes = test_alias().encode();
        assert_eq!(
            old_bytes, new_bytes,
            "new AliasV2::encode must produce byte-identical output to build_background_alias"
        );
    }
}
