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

/// Canonical background image filename inside the DMG's `.background/` folder.
pub const DMG_BG_FILENAME: &str = "bg.png";

/// A complete `.DS_Store` file: a set of records that encode to the buddy-allocator B-tree format.
#[derive(Debug, Clone, PartialEq)]
pub struct DsStore {
    pub(crate) records: Vec<DsRecord>,
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
        // Build will be implemented in encode.rs Task 7.
        // For now, return an empty DsStore so the module compiles.
        todo!("DsStoreBuilder::build — implemented in Task 7")
    }
}
