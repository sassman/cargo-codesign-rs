//! BinaryDecode implementations for DS_Store record types.

use super::alias::AliasV2;
use super::bookmark::Bookmark;
use super::types::{
    BinaryDecode, DecodeError, DsRecord, IconLocation, IconViewSettings, RecordValue,
    WindowSettings,
};

// ---------------------------------------------------------------------------
// IconLocation
// ---------------------------------------------------------------------------

impl BinaryDecode for IconLocation {
    fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < 16 {
            return Err(DecodeError::TooShort {
                expected: 16,
                got: data.len(),
            });
        }
        let x = u32::from_be_bytes(read4(data, 0));
        let y = u32::from_be_bytes(read4(data, 4));
        // Bytes 8..16 are padding — ignored on decode.
        Ok(IconLocation { x, y })
    }
}

// ---------------------------------------------------------------------------
// WindowSettings
// ---------------------------------------------------------------------------

impl BinaryDecode for WindowSettings {
    fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        let dict: plist::Dictionary =
            plist::from_bytes(data).map_err(|e| DecodeError::Plist(e.to_string()))?;

        let bounds_str = dict
            .get("WindowBounds")
            .and_then(plist::Value::as_string)
            .ok_or_else(|| DecodeError::Other("missing WindowBounds key".into()))?;

        let (origin, size) = parse_window_bounds(bounds_str)?;

        let show_sidebar = dict
            .get("ShowSidebar")
            .and_then(plist::Value::as_boolean)
            .unwrap_or(false);
        let container_show_sidebar = dict
            .get("ContainerShowSidebar")
            .and_then(plist::Value::as_boolean)
            .unwrap_or(false);
        let show_toolbar = dict
            .get("ShowToolbar")
            .and_then(plist::Value::as_boolean)
            .unwrap_or(false);
        let show_tab_view = dict
            .get("ShowTabView")
            .and_then(plist::Value::as_boolean)
            .unwrap_or(false);
        let show_status_bar = dict
            .get("ShowStatusBar")
            .and_then(plist::Value::as_boolean)
            .unwrap_or(false);

        Ok(WindowSettings {
            window_origin: origin,
            window_width: size.0,
            window_height: size.1,
            show_sidebar,
            container_show_sidebar,
            show_toolbar,
            show_tab_view,
            show_status_bar,
        })
    }
}

/// Parse `"{{x, y}, {w, h}}"` into `((x, y), (w, h))`.
fn parse_window_bounds(s: &str) -> Result<((u32, u32), (u32, u32)), DecodeError> {
    // Strip outer braces and whitespace, then parse the four integers.
    let stripped = s
        .replace('{', "")
        .replace('}', "");
    let parts: Vec<&str> = stripped.split(',').map(str::trim).collect();
    if parts.len() != 4 {
        return Err(DecodeError::Other(format!(
            "WindowBounds has {n} components, expected 4: {s}",
            n = parts.len()
        )));
    }
    let x = parse_u32(parts[0], s)?;
    let y = parse_u32(parts[1], s)?;
    let w = parse_u32(parts[2], s)?;
    let h = parse_u32(parts[3], s)?;
    Ok(((x, y), (w, h)))
}

fn parse_u32(s: &str, context: &str) -> Result<u32, DecodeError> {
    s.parse::<u32>().map_err(|e| {
        DecodeError::Other(format!("invalid integer '{s}' in WindowBounds '{context}': {e}"))
    })
}

// ---------------------------------------------------------------------------
// IconViewSettings
// ---------------------------------------------------------------------------

impl BinaryDecode for IconViewSettings {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        let dict: plist::Dictionary =
            plist::from_bytes(data).map_err(|e| DecodeError::Plist(e.to_string()))?;

        let icon_size = plist_real_or_int(&dict, "iconSize")? as u32;
        let text_size = plist_real_or_int(&dict, "textSize")?;
        let label_on_bottom = plist_bool(&dict, "labelOnBottom", true);
        let show_icon_preview = plist_bool(&dict, "showIconPreview", true);
        let show_item_info = plist_bool(&dict, "showItemInfo", false);
        let arrange_by = dict
            .get("arrangeBy")
            .and_then(plist::Value::as_string)
            .unwrap_or("none")
            .to_string();
        let grid_spacing = plist_real_or_int(&dict, "gridSpacing").unwrap_or(100.0);
        let grid_offset_x = plist_real_or_int(&dict, "gridOffsetX").unwrap_or(0.0);
        let grid_offset_y = plist_real_or_int(&dict, "gridOffsetY").unwrap_or(0.0);
        let view_options_version = plist_real_or_int(&dict, "viewOptionsVersion")
            .unwrap_or(1.0) as u32;
        let background_type = plist_real_or_int(&dict, "backgroundType")
            .unwrap_or(1.0) as u32;

        let bg_red = plist_real_or_int(&dict, "backgroundColorRed").unwrap_or(1.0);
        let bg_green = plist_real_or_int(&dict, "backgroundColorGreen").unwrap_or(1.0);
        let bg_blue = plist_real_or_int(&dict, "backgroundColorBlue").unwrap_or(1.0);

        let alias_bytes = dict
            .get("backgroundImageAlias")
            .and_then(plist::Value::as_data)
            .ok_or_else(|| DecodeError::Other("missing backgroundImageAlias".into()))?;

        let background_alias = AliasV2::decode(alias_bytes)?;

        Ok(IconViewSettings {
            icon_size,
            text_size,
            label_on_bottom,
            show_icon_preview,
            show_item_info,
            arrange_by,
            grid_spacing,
            grid_offset_x,
            grid_offset_y,
            view_options_version,
            background_type,
            background_color: (bg_red, bg_green, bg_blue),
            background_alias,
        })
    }
}

/// Extract a numeric plist value that may be stored as Real or Integer.
fn plist_real_or_int(dict: &plist::Dictionary, key: &str) -> Result<f64, DecodeError> {
    let val = dict
        .get(key)
        .ok_or_else(|| DecodeError::Other(format!("missing plist key '{key}'")))?;
    match val {
        plist::Value::Real(r) => Ok(*r),
        plist::Value::Integer(i) => {
            i.as_signed()
                .map(|v| v as f64)
                .ok_or_else(|| DecodeError::Other(format!("integer overflow in plist key '{key}'")))
        }
        _ => Err(DecodeError::Other(format!(
            "expected real or integer for plist key '{key}', got {val:?}"
        ))),
    }
}

/// Extract a boolean plist value, returning `default` if missing.
fn plist_bool(dict: &plist::Dictionary, key: &str, default: bool) -> bool {
    dict.get(key)
        .and_then(plist::Value::as_boolean)
        .unwrap_or(default)
}

// ---------------------------------------------------------------------------
// DsRecord::decode_one
// ---------------------------------------------------------------------------

impl DsRecord {
    /// Decode one record from a packed byte slice.
    ///
    /// Returns the decoded record and the total number of bytes consumed,
    /// so the caller can advance through sequentially packed records.
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn decode_one(data: &[u8]) -> Result<(Self, usize), DecodeError> {
        let mut pos = 0;

        // filename length in UTF-16 code units
        if data.len() < pos + 4 {
            return Err(DecodeError::TooShort {
                expected: 4,
                got: data.len(),
            });
        }
        let filename_len = u32::from_be_bytes(read4(data, pos)) as usize;
        pos += 4;

        // UTF-16 BE filename
        let utf16_byte_len = filename_len * 2;
        if data.len() < pos + utf16_byte_len {
            return Err(DecodeError::TooShort {
                expected: pos + utf16_byte_len,
                got: data.len(),
            });
        }
        let mut utf16_units = Vec::with_capacity(filename_len);
        for i in 0..filename_len {
            let offset = pos + i * 2;
            utf16_units.push(u16::from_be_bytes([data[offset], data[offset + 1]]));
        }
        let filename = String::from_utf16(&utf16_units)
            .map_err(|e| DecodeError::Other(format!("invalid UTF-16 filename: {e}")))?;
        pos += utf16_byte_len;

        // record code (4 bytes)
        if data.len() < pos + 4 {
            return Err(DecodeError::TooShort {
                expected: pos + 4,
                got: data.len(),
            });
        }
        let mut record_code = [0u8; 4];
        record_code.copy_from_slice(&data[pos..pos + 4]);
        pos += 4;

        // type tag (4 bytes)
        if data.len() < pos + 4 {
            return Err(DecodeError::TooShort {
                expected: pos + 4,
                got: data.len(),
            });
        }
        let mut type_tag = [0u8; 4];
        type_tag.copy_from_slice(&data[pos..pos + 4]);
        pos += 4;

        // payload
        let value = match (&record_code, &type_tag) {
            (b"Iloc", b"blob") => {
                let (blob, blob_len) = read_blob(data, pos)?;
                pos += 4 + blob_len;
                RecordValue::Iloc(IconLocation::decode(blob)?)
            }
            (b"bwsp", b"blob") => {
                let (blob, blob_len) = read_blob(data, pos)?;
                pos += 4 + blob_len;
                RecordValue::Bwsp(WindowSettings::decode(blob)?)
            }
            (b"icvp", b"blob") => {
                let (blob, blob_len) = read_blob(data, pos)?;
                pos += 4 + blob_len;
                RecordValue::Icvp(IconViewSettings::decode(blob)?)
            }
            (b"pBBk", b"blob") => {
                let (blob, blob_len) = read_blob(data, pos)?;
                pos += 4 + blob_len;
                RecordValue::PBBk(Bookmark::decode(blob)?)
            }
            (b"vSrn", b"long") => {
                if data.len() < pos + 4 {
                    return Err(DecodeError::TooShort {
                        expected: pos + 4,
                        got: data.len(),
                    });
                }
                let v = i32::from_be_bytes(read4(data, pos));
                pos += 4;
                RecordValue::VSrn(v)
            }
            (_, b"blob") => {
                let (blob, blob_len) = read_blob(data, pos)?;
                pos += 4 + blob_len;
                RecordValue::Unknown {
                    code: record_code,
                    type_tag,
                    data: blob.to_vec(),
                }
            }
            (_, b"long") => {
                if data.len() < pos + 4 {
                    return Err(DecodeError::TooShort {
                        expected: pos + 4,
                        got: data.len(),
                    });
                }
                let raw = data[pos..pos + 4].to_vec();
                pos += 4;
                RecordValue::Unknown {
                    code: record_code,
                    type_tag,
                    data: raw,
                }
            }
            (_, b"bool") => {
                if data.len() < pos + 1 {
                    return Err(DecodeError::TooShort {
                        expected: pos + 1,
                        got: data.len(),
                    });
                }
                let raw = vec![data[pos]];
                pos += 1;
                RecordValue::Unknown {
                    code: record_code,
                    type_tag,
                    data: raw,
                }
            }
            _ => {
                return Err(DecodeError::InvalidTypeTag(type_tag));
            }
        };

        Ok((DsRecord { filename, value }, pos))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read 4 bytes at `offset` as a fixed-size array.
fn read4(data: &[u8], offset: usize) -> [u8; 4] {
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&data[offset..offset + 4]);
    buf
}

/// Read a blob: 4-byte BE length prefix, then that many bytes of data.
/// Returns `(data_slice, data_len)`. The caller must advance by `4 + data_len`.
fn read_blob(data: &[u8], pos: usize) -> Result<(&[u8], usize), DecodeError> {
    if data.len() < pos + 4 {
        return Err(DecodeError::TooShort {
            expected: pos + 4,
            got: data.len(),
        });
    }
    let blob_len = u32::from_be_bytes(read4(data, pos)) as usize;
    let blob_start = pos + 4;
    let blob_end = blob_start + blob_len;
    if data.len() < blob_end {
        return Err(DecodeError::TooShort {
            expected: blob_end,
            got: data.len(),
        });
    }
    Ok((&data[blob_start..blob_end], blob_len))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds_store::encode::*;
    use crate::ds_store::types::*;

    #[test]
    fn iloc_decode_roundtrip() {
        let iloc = IconLocation { x: 500, y: 200 };
        let bytes = iloc.encode();
        let decoded = IconLocation::decode(&bytes).unwrap();
        assert_eq!(decoded.x, 500);
        assert_eq!(decoded.y, 200);
    }

    #[test]
    fn iloc_record_roundtrip() {
        let rec = DsRecord {
            filename: "Applications".to_string(),
            value: RecordValue::Iloc(IconLocation { x: 500, y: 200 }),
        };
        let bytes = rec.encode();
        let (decoded, consumed) = DsRecord::decode_one(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(decoded.filename, "Applications");
        if let RecordValue::Iloc(iloc) = &decoded.value {
            assert_eq!(iloc.x, 500);
            assert_eq!(iloc.y, 200);
        } else {
            panic!("expected Iloc");
        }
    }

    #[test]
    fn vsrn_record_roundtrip() {
        let rec = DsRecord {
            filename: ".".to_string(),
            value: RecordValue::VSrn(1),
        };
        let bytes = rec.encode();
        let (decoded, consumed) = DsRecord::decode_one(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(decoded.filename, ".");
        assert!(matches!(decoded.value, RecordValue::VSrn(1)));
    }

    #[test]
    fn window_settings_decode_roundtrip() {
        let ws = WindowSettings {
            window_origin: (200, 120),
            window_width: 660,
            window_height: 400,
            show_sidebar: false,
            container_show_sidebar: false,
            show_toolbar: false,
            show_tab_view: false,
            show_status_bar: false,
        };
        let bytes = ws.encode();
        let decoded = WindowSettings::decode(&bytes).unwrap();
        assert_eq!(decoded.window_width, 660);
        assert_eq!(decoded.window_height, 400);
        assert!(!decoded.show_sidebar);
    }
}
