//! Type definitions, traits, and error types for the DS_Store binary format.

use std::fmt;

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

pub(crate) trait BinaryEncode {
    fn encode(&self) -> Vec<u8>;
}

pub(crate) trait BinaryDecode: Sized {
    fn decode(data: &[u8]) -> Result<Self, DecodeError>;
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) enum DecodeError {
    TooShort { expected: usize, got: usize },
    InvalidMagic { expected: &'static [u8], got: Vec<u8> },
    InvalidRecordCode([u8; 4]),
    InvalidTypeTag([u8; 4]),
    Plist(String),
    Other(String),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { expected, got } => {
                write!(f, "data too short: expected {expected} bytes, got {got}")
            }
            Self::InvalidMagic { expected, got } => {
                write!(f, "invalid magic: expected {expected:?}, got {got:?}")
            }
            Self::InvalidRecordCode(code) => write!(f, "unknown record code: {code:?}"),
            Self::InvalidTypeTag(tag) => write!(f, "unknown type tag: {tag:?}"),
            Self::Plist(msg) => write!(f, "plist error: {msg}"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for DecodeError {}

// ---------------------------------------------------------------------------
// Record types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DsRecord {
    pub(crate) filename: String,
    pub(crate) value: RecordValue,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RecordValue {
    Iloc(IconLocation),
    Bwsp(WindowSettings),
    Icvp(IconViewSettings),
    PBBk(Bookmark),
    VSrn(i32),
    Unknown {
        code: [u8; 4],
        type_tag: [u8; 4],
        data: Vec<u8>,
    },
}

impl RecordValue {
    pub(crate) fn record_code(&self) -> [u8; 4] {
        match self {
            Self::Iloc(_) => *b"Iloc",
            Self::Bwsp(_) => *b"bwsp",
            Self::Icvp(_) => *b"icvp",
            Self::PBBk(_) => *b"pBBk",
            Self::VSrn(_) => *b"vSrn",
            Self::Unknown { code, .. } => *code,
        }
    }

    pub(crate) fn type_tag(&self) -> [u8; 4] {
        match self {
            Self::Iloc(_) | Self::Bwsp(_) | Self::Icvp(_) | Self::PBBk(_) => *b"blob",
            Self::VSrn(_) => *b"long",
            Self::Unknown { type_tag, .. } => *type_tag,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct IconLocation {
    pub(crate) x: u32,
    pub(crate) y: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WindowSettings {
    pub(crate) window_origin: (u32, u32),
    pub(crate) window_width: u32,
    pub(crate) window_height: u32,
    pub(crate) show_sidebar: bool,
    pub(crate) container_show_sidebar: bool,
    pub(crate) show_toolbar: bool,
    pub(crate) show_tab_view: bool,
    pub(crate) show_status_bar: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct IconViewSettings {
    pub(crate) icon_size: u32,
    pub(crate) text_size: f64,
    pub(crate) label_on_bottom: bool,
    pub(crate) show_icon_preview: bool,
    pub(crate) show_item_info: bool,
    pub(crate) arrange_by: String,
    pub(crate) grid_spacing: f64,
    pub(crate) grid_offset_x: f64,
    pub(crate) grid_offset_y: f64,
    pub(crate) view_options_version: u32,
    pub(crate) background_type: u32,
    pub(crate) background_color: (f64, f64, f64),
    pub(crate) background_alias: AliasV2,
}

// Forward-declare types that live in other files but are needed here
pub(crate) use super::alias::{AliasKind, AliasTag, AliasV2};
pub(crate) use super::bookmark::Bookmark;
