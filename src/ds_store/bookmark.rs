//! macOS Bookmark binary format — encode and decode.
//! Full implementation in Task 3.

use super::types::{BinaryEncode, DecodeError};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Bookmark {
    pub(crate) path_components: Vec<String>,
    pub(crate) volume_name: String,
    pub(crate) volume_path: String,
    pub(crate) volume_url: String,
    pub(crate) volume_uuid: String,
    pub(crate) volume_capacity: u64,
}

impl BinaryEncode for Bookmark {
    fn encode(&self) -> Vec<u8> { todo!("Task 3") }
}
