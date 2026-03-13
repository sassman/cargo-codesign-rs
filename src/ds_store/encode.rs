//! `BinaryEncode` implementations for `DS_Store` record types.

use plist::Value as PlistValue;
use std::collections::BTreeMap;

use super::types::{
    BinaryEncode, DsRecord, IconLocation, IconViewSettings, RecordValue, WindowSettings,
};

// ---------------------------------------------------------------------------
// IconLocation
// ---------------------------------------------------------------------------

impl BinaryEncode for IconLocation {
    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(16);
        out.extend_from_slice(&self.x.to_be_bytes());
        out.extend_from_slice(&self.y.to_be_bytes());
        out.extend_from_slice(&0xFFFF_FFFFu32.to_be_bytes());
        out.extend_from_slice(&0xFFFF_0000u32.to_be_bytes());
        out
    }
}

// ---------------------------------------------------------------------------
// WindowSettings
// ---------------------------------------------------------------------------

impl BinaryEncode for WindowSettings {
    fn encode(&self) -> Vec<u8> {
        let (ox, oy) = self.window_origin;
        let mut dict = BTreeMap::new();
        dict.insert(
            "ContainerShowSidebar".to_string(),
            PlistValue::Boolean(self.container_show_sidebar),
        );
        dict.insert(
            "ShowSidebar".to_string(),
            PlistValue::Boolean(self.show_sidebar),
        );
        dict.insert(
            "ShowStatusBar".to_string(),
            PlistValue::Boolean(self.show_status_bar),
        );
        dict.insert(
            "ShowTabView".to_string(),
            PlistValue::Boolean(self.show_tab_view),
        );
        dict.insert(
            "ShowToolbar".to_string(),
            PlistValue::Boolean(self.show_toolbar),
        );
        dict.insert(
            "WindowBounds".to_string(),
            PlistValue::String(format!(
                "{{{{{ox}, {oy}}}, {{{}, {}}}}}",
                self.window_width, self.window_height
            )),
        );
        let val = PlistValue::Dictionary(dict.into_iter().collect());
        let mut buf = Vec::new();
        val.to_writer_binary(&mut buf)
            .expect("plist serialization should not fail for known-good data");
        buf
    }
}

// ---------------------------------------------------------------------------
// IconViewSettings
// ---------------------------------------------------------------------------

impl BinaryEncode for IconViewSettings {
    fn encode(&self) -> Vec<u8> {
        let mut dict: BTreeMap<String, PlistValue> = BTreeMap::new();
        dict.insert(
            "arrangeBy".to_string(),
            PlistValue::String(self.arrange_by.clone()),
        );
        let (r, g, b) = self.background_color;
        dict.insert("backgroundColorBlue".to_string(), PlistValue::Real(b));
        dict.insert("backgroundColorGreen".to_string(), PlistValue::Real(g));
        dict.insert("backgroundColorRed".to_string(), PlistValue::Real(r));
        dict.insert(
            "backgroundImageAlias".to_string(),
            PlistValue::Data(self.background_alias.encode()),
        );
        dict.insert(
            "backgroundType".to_string(),
            PlistValue::Integer(self.background_type.into()),
        );
        dict.insert(
            "gridOffsetX".to_string(),
            PlistValue::Real(self.grid_offset_x),
        );
        dict.insert(
            "gridOffsetY".to_string(),
            PlistValue::Real(self.grid_offset_y),
        );
        dict.insert(
            "gridSpacing".to_string(),
            PlistValue::Real(self.grid_spacing),
        );
        dict.insert(
            "iconSize".to_string(),
            PlistValue::Real(f64::from(self.icon_size)),
        );
        dict.insert(
            "labelOnBottom".to_string(),
            PlistValue::Boolean(self.label_on_bottom),
        );
        dict.insert(
            "showIconPreview".to_string(),
            PlistValue::Boolean(self.show_icon_preview),
        );
        dict.insert(
            "showItemInfo".to_string(),
            PlistValue::Boolean(self.show_item_info),
        );
        dict.insert("textSize".to_string(), PlistValue::Real(self.text_size));
        dict.insert(
            "viewOptionsVersion".to_string(),
            PlistValue::Integer(self.view_options_version.into()),
        );

        let val = PlistValue::Dictionary(dict.into_iter().collect());
        let mut buf = Vec::new();
        val.to_writer_binary(&mut buf)
            .expect("plist serialization should not fail for known-good data");
        buf
    }
}

// ---------------------------------------------------------------------------
// RecordValue
// ---------------------------------------------------------------------------

impl BinaryEncode for RecordValue {
    #[allow(clippy::cast_possible_truncation)]
    fn encode(&self) -> Vec<u8> {
        match self {
            Self::Iloc(iloc) => blob_wrap(&iloc.encode()),
            Self::Bwsp(ws) => blob_wrap(&ws.encode()),
            Self::Icvp(ivs) => blob_wrap(&ivs.encode()),
            Self::PBBk(bk) => blob_wrap(&bk.encode()),
            Self::VSrn(v) => v.to_be_bytes().to_vec(),
            Self::Unknown { type_tag, data, .. } => {
                if type_tag == b"blob" {
                    blob_wrap(data)
                } else {
                    data.clone()
                }
            }
        }
    }
}

/// Wrap blob data with a 4-byte big-endian length prefix.
#[allow(clippy::cast_possible_truncation)]
fn blob_wrap(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + data.len());
    // Blob lengths in DS_Store are u32; our blobs are always < 4 GiB.
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(data);
    out
}

// ---------------------------------------------------------------------------
// DsRecord
// ---------------------------------------------------------------------------

impl BinaryEncode for DsRecord {
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
        out.extend_from_slice(&self.value.record_code());
        out.extend_from_slice(&self.value.type_tag());
        out.extend_from_slice(&self.value.encode());
        out
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds_store::alias::*;
    use crate::ds_store::types::*;

    #[test]
    fn icon_location_encodes_to_16_bytes() {
        let iloc = IconLocation { x: 160, y: 200 };
        let bytes = iloc.encode();
        assert_eq!(bytes.len(), 16);
        assert_eq!(u32::from_be_bytes(bytes[0..4].try_into().unwrap()), 160);
        assert_eq!(u32::from_be_bytes(bytes[4..8].try_into().unwrap()), 200);
        assert_eq!(
            u32::from_be_bytes(bytes[8..12].try_into().unwrap()),
            0xFFFFFFFF
        );
        assert_eq!(
            u32::from_be_bytes(bytes[12..16].try_into().unwrap()),
            0xFFFF0000
        );
    }

    #[test]
    fn window_settings_encode_is_valid_plist() {
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
        assert!(bytes.starts_with(b"bplist"));
        let parsed: plist::Dictionary = plist::from_bytes(&bytes).unwrap();
        assert_eq!(
            parsed.get("WindowBounds").and_then(|v| v.as_string()),
            Some("{{200, 120}, {660, 400}}")
        );
    }

    #[test]
    fn vsrn_record_encoding() {
        let rec = DsRecord {
            filename: ".".to_string(),
            value: RecordValue::VSrn(1),
        };
        let bytes = rec.encode();
        // filename_len(4) + "."_utf16(2) + code(4) + type(4) + i32(4) = 18
        assert_eq!(bytes.len(), 18);
        assert_eq!(u32::from_be_bytes(bytes[0..4].try_into().unwrap()), 1); // 1 char
    }
}
