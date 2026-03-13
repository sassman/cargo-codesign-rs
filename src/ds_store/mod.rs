//! Typed `.DS_Store` file format — encode and decode.
//!
//! Replaces manual byte-slice manipulation with typed Rust structs.
//! The module generates binary `.DS_Store` files for macOS DMG installers.

mod alias;
mod allocator;
mod bookmark;
mod decode;
mod encode;
mod types;

pub(crate) use types::*;

use alias::{AliasKind, AliasTag, AliasV2};
use allocator::{block_address, log2, next_power_of_two, AllocatorInfo, Bud1Prelude, Dsdb};
use bookmark::Bookmark;

/// Canonical background image filename inside the DMG's `.background/` folder.
pub const DMG_BG_FILENAME: &str = "bg.png";

/// A complete `.DS_Store` file: a set of records that encode to the buddy-allocator B-tree format.
#[derive(Debug, Clone, PartialEq)]
pub struct DsStore {
    pub(crate) records: Vec<DsRecord>,
}

impl DsStore {
    /// Encode the `DsStore` into a complete `.DS_Store` binary file.
    ///
    /// The layout matches the buddy-allocator B-tree format that Finder expects:
    /// one DSDB block, one leaf node, and an allocator info block.
    #[allow(clippy::cast_possible_truncation)]
    pub fn encode(&self) -> Vec<u8> {
        let num_records = self.records.len() as u32;

        // --- Serialize the two data blocks ---
        let leaf_data = serialize_leaf_node(&self.records);
        let dsdb_placeholder = Dsdb {
            root_node: 0,
            num_records,
        };
        let dsdb_data_placeholder = dsdb_placeholder.encode();

        // --- Compute block sizes (power-of-two, minimum 32) ---
        let dsdb_alloc = next_power_of_two(dsdb_data_placeholder.len());
        let leaf_alloc = next_power_of_two(leaf_data.len());

        let dsdb_log2 = log2(dsdb_alloc);
        let leaf_log2 = log2(leaf_alloc);

        // --- Layout the data region ---
        //
        // Data region starts at file offset 4 (right after the 4-byte file header).
        // All offsets below are relative to byte 4.
        //
        //   data_offset 0..32           = Bud1 prelude (not a block)
        //   data_offset 32..32+dsdb     = DSDB block       -> block[1]
        //   data_offset ...             = leaf node block   -> block[2]
        //   data_offset ...             = allocator info    -> block[0]
        let dsdb_offset: u32 = 32;
        let leaf_offset: u32 = dsdb_offset + dsdb_alloc as u32;
        let info_offset: u32 = leaf_offset + leaf_alloc as u32;

        let dsdb_addr = block_address(dsdb_offset, dsdb_log2);
        let leaf_addr = block_address(leaf_offset, leaf_log2);

        // Re-serialize DSDB with correct root_node = 2 (leaf is block index 2)
        let dsdb_data = Dsdb {
            root_node: 2,
            num_records,
        }
        .encode();

        // Allocator info needs to know its own address for block[0].
        // We compute info_alloc first, then build the block.
        let info_alloc = next_power_of_two(1200); // allocator info is ~1169 bytes; always 2048
        let info_log2 = log2(info_alloc);
        let info_addr = block_address(info_offset, info_log2);

        let allocator_info = AllocatorInfo {
            block_addresses: vec![info_addr, dsdb_addr, leaf_addr],
            toc: vec![("DSDB".to_string(), 1)],
        };
        let info_block = allocator_info.encode();

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

        // Prelude (32 bytes in the data region)
        let prelude = Bud1Prelude {
            info_offset: info_offset,
            info_alloc: info_alloc as u32,
            leaf_addr,
        };
        file.extend_from_slice(&prelude.encode());

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
}

/// Serialize a B-tree leaf node containing the given records.
///
/// Leaf node layout:
///   - `pair_count`: 0 (u32 BE) -- signals this is a leaf
///   - `record_count` (u32 BE)
///   - serialized records
#[allow(clippy::cast_possible_truncation)]
fn serialize_leaf_node(records: &[DsRecord]) -> Vec<u8> {
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

/// Builder for constructing a `DsStore` for a DMG installer layout.
pub struct DsStoreBuilder {
    window_width: u32,
    window_height: u32,
    icon_size: u32,
    app_name: String,
    app_position: (u32, u32),
    apps_link_position: (u32, u32),
    volume_name: String,
}

impl DsStoreBuilder {
    pub fn new(app_name: impl Into<String>, volume_name: impl Into<String>) -> Self {
        Self {
            window_width: 660,
            window_height: 400,
            icon_size: 128,
            app_name: app_name.into(),
            app_position: (160, 200),
            apps_link_position: (500, 200),
            volume_name: volume_name.into(),
        }
    }

    pub fn window_size(mut self, width: u32, height: u32) -> Self {
        self.window_width = width;
        self.window_height = height;
        self
    }

    pub fn icon_size(mut self, size: u32) -> Self {
        self.icon_size = size;
        self
    }

    pub fn app_position(mut self, x: u32, y: u32) -> Self {
        self.app_position = (x, y);
        self
    }

    pub fn apps_link_position(mut self, x: u32, y: u32) -> Self {
        self.apps_link_position = (x, y);
        self
    }

    /// Build the `DsStore`. The background filename is always [`DMG_BG_FILENAME`].
    pub fn build(self) -> DsStore {
        let alias = AliasV2 {
            kind: AliasKind::File,
            volume_name: self.volume_name.clone(),
            volume_created: 0,
            volume_signature: *b"H+",
            volume_type: 5,
            parent_dir_id: 0,
            filename: DMG_BG_FILENAME.to_string(),
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
                AliasTag::UnicodeFilename(DMG_BG_FILENAME.to_string()),
                AliasTag::UnicodeVolumeName(self.volume_name.clone()),
                AliasTag::PosixPath(format!("/.background/{}", DMG_BG_FILENAME)),
                AliasTag::VolumeMountPoint(format!("/Volumes/{}", self.volume_name)),
            ],
        };

        let bookmark = Bookmark {
            path_components: vec![
                "Volumes".to_string(),
                self.volume_name.clone(),
                ".background".to_string(),
                DMG_BG_FILENAME.to_string(),
            ],
            volume_name: self.volume_name.clone(),
            volume_path: format!("/Volumes/{}", self.volume_name),
            volume_url: format!("file:///Volumes/{}/", self.volume_name),
            volume_uuid: "00000000-0000-0000-0000-000000000000".to_string(),
            volume_capacity: 52_428_800,
        };

        let mut records = vec![
            // bwsp: window settings for volume root "."
            DsRecord {
                filename: ".".to_string(),
                value: RecordValue::Bwsp(WindowSettings {
                    window_origin: (200, 120),
                    window_width: self.window_width,
                    window_height: self.window_height,
                    show_sidebar: false,
                    container_show_sidebar: false,
                    show_toolbar: false,
                    show_tab_view: false,
                    show_status_bar: false,
                }),
            },
            // icvp: icon view settings for volume root "."
            DsRecord {
                filename: ".".to_string(),
                value: RecordValue::Icvp(IconViewSettings {
                    icon_size: self.icon_size,
                    text_size: 12.0,
                    label_on_bottom: true,
                    show_icon_preview: true,
                    show_item_info: false,
                    arrange_by: "none".to_string(),
                    grid_spacing: 100.0,
                    grid_offset_x: 0.0,
                    grid_offset_y: 0.0,
                    view_options_version: 1,
                    background_type: 2,
                    background_color: (1.0, 1.0, 1.0),
                    background_alias: alias,
                }),
            },
            // pBBk: bookmark for volume root "."
            DsRecord {
                filename: ".".to_string(),
                value: RecordValue::PBBk(bookmark),
            },
            // vSrn(1) for volume root "."
            DsRecord {
                filename: ".".to_string(),
                value: RecordValue::VSrn(1),
            },
            // Iloc: Applications symlink position
            DsRecord {
                filename: "Applications".to_string(),
                value: RecordValue::Iloc(IconLocation {
                    x: self.apps_link_position.0,
                    y: self.apps_link_position.1,
                }),
            },
            // Iloc: app icon position
            DsRecord {
                filename: self.app_name.clone(),
                value: RecordValue::Iloc(IconLocation {
                    x: self.app_position.0,
                    y: self.app_position.1,
                }),
            },
        ];

        // Sort by (filename as UTF-16 code units, then record code bytes).
        records.sort_by(|a, b| {
            let a_utf16: Vec<u16> = a.filename.encode_utf16().collect();
            let b_utf16: Vec<u16> = b.filename.encode_utf16().collect();
            a_utf16
                .cmp(&b_utf16)
                .then_with(|| a.value.record_code().cmp(&b.value.record_code()))
        });

        DsStore { records }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ds_store() -> DsStore {
        DsStoreBuilder::new("JPEG Locker.app", "JPEG Locker")
            .window_size(660, 400)
            .icon_size(128)
            .app_position(160, 200)
            .apps_link_position(500, 200)
            .build()
    }

    #[test]
    fn builder_produces_byte_identical_output() {
        let old = crate::ds_store_old::write_ds_store(
            &crate::ds_store_old::DmgLayout {
                window_width: 660,
                window_height: 400,
                icon_size: 128,
                app_name: "JPEG Locker.app".to_string(),
                app_x: 160,
                app_y: 200,
                apps_link_x: 500,
                apps_link_y: 200,
                background_filename: "bg.png".to_string(),
                volume_name: "JPEG Locker".to_string(),
            },
        );
        let new = test_ds_store().encode();

        assert_eq!(
            old.len(),
            new.len(),
            "length mismatch: old={} new={}",
            old.len(),
            new.len()
        );

        // Find first divergence for a useful error message.
        for (i, (a, b)) in old.iter().zip(new.iter()).enumerate() {
            if a != b {
                panic!("first diff at byte {i}: old=0x{a:02x} new=0x{b:02x}");
            }
        }

        assert_eq!(old, new, "new module must produce byte-identical output");
    }

    #[test]
    fn output_starts_with_header_and_magic() {
        let bytes = test_ds_store().encode();
        assert_eq!(u32::from_be_bytes(bytes[0..4].try_into().unwrap()), 1);
        assert_eq!(&bytes[4..8], b"Bud1");
    }

    #[test]
    fn output_contains_record_codes() {
        let bytes = test_ds_store().encode();
        let has_pattern = |pat: &[u8]| bytes.windows(pat.len()).any(|w| w == pat);
        assert!(has_pattern(b"Iloc"));
        assert!(has_pattern(b"bwsp"));
        assert!(has_pattern(b"icvp"));
        assert!(has_pattern(b"vSrn"));
        assert!(has_pattern(b"pBBk"));
    }
}
