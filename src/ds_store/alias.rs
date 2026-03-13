//! Alias V2 binary format — encode and decode.
//! Full implementation in Task 2.

use super::types::{BinaryEncode, DecodeError};

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

impl BinaryEncode for AliasV2 {
    fn encode(&self) -> Vec<u8> { todo!("Task 2") }
}
