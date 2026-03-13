//! Buddy allocator primitives — `Bud1Prelude`, Dsdb, `AllocatorInfo`.

use super::types::{BinaryDecode, BinaryEncode, DecodeError};

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

pub(crate) struct Bud1Prelude {
    pub(crate) info_offset: u32,
    pub(crate) info_alloc: u32,
    pub(crate) leaf_addr: u32,
}

pub(crate) struct Dsdb {
    pub(crate) root_node: u32,
    pub(crate) num_records: u32,
}

pub(crate) struct AllocatorInfo {
    pub(crate) block_addresses: Vec<u32>,
    pub(crate) toc: Vec<(String, u32)>,
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Pack an aligned data-region offset and a log2 size class into a single u32.
///
/// The lower 5 bits hold the size class; the upper bits hold the offset
/// (which must already be 32-byte aligned).
pub(crate) fn block_address(offset: u32, size_class: u32) -> u32 {
    debug_assert!(size_class < 32, "size_class must fit in 5 bits");
    debug_assert!(
        offset.trailing_zeros() >= 5,
        "offset must be aligned to 32 bytes"
    );
    (offset & !0x1f) | size_class
}

/// Round `size` up to the next power of two, with a minimum of 32.
pub(crate) fn next_power_of_two(size: usize) -> usize {
    let min = 32;
    let v = size.max(min);
    v.next_power_of_two()
}

/// Return log2 of a power-of-two value.
pub(crate) fn log2(v: usize) -> u32 {
    debug_assert!(v.is_power_of_two());
    v.trailing_zeros()
}

// ---------------------------------------------------------------------------
// Encode
// ---------------------------------------------------------------------------

impl BinaryEncode for Bud1Prelude {
    /// Encode to 32 bytes:
    /// `"Bud1"(4) + info_offset(4) + info_alloc(4) + info_offset(4, dup) + leaf_addr(4) + zeros(12)`
    fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(32);
        buf.extend_from_slice(b"Bud1");
        buf.extend_from_slice(&self.info_offset.to_be_bytes());
        buf.extend_from_slice(&self.info_alloc.to_be_bytes());
        buf.extend_from_slice(&self.info_offset.to_be_bytes());
        buf.extend_from_slice(&self.leaf_addr.to_be_bytes());
        buf.extend_from_slice(&[0u8; 12]);
        buf
    }
}

impl BinaryEncode for Dsdb {
    /// Encode to 20 bytes:
    /// `root_node(4) + 0(4) + num_records(4) + 1(4) + 0x1000(4)`
    fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(20);
        buf.extend_from_slice(&self.root_node.to_be_bytes());
        buf.extend_from_slice(&0u32.to_be_bytes());
        buf.extend_from_slice(&self.num_records.to_be_bytes());
        buf.extend_from_slice(&1u32.to_be_bytes());
        buf.extend_from_slice(&0x0000_1000u32.to_be_bytes());
        buf
    }
}

impl BinaryEncode for AllocatorInfo {
    /// Encode the allocator info block.
    ///
    /// Layout:
    /// - `num_offsets(4)` + `0(4)` (reserved)
    /// - 256 x `addr(4)` — block addresses padded with zeros to 256 entries
    /// - `toc_count(4)` + for each: `name_len(1)` + `name_bytes` + `value(4)`
    /// - 32 x `0(4)` — free list (128 bytes)
    fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(2048);

        // DS_Store files have at most a handful of blocks; truncation is safe.
        #[allow(clippy::cast_possible_truncation)]
        let num_offsets = self.block_addresses.len() as u32;
        buf.extend_from_slice(&num_offsets.to_be_bytes());
        buf.extend_from_slice(&0u32.to_be_bytes());

        // Block address array: actual entries followed by zero-padding to 256
        for &addr in &self.block_addresses {
            buf.extend_from_slice(&addr.to_be_bytes());
        }
        let padding_count = 256 - self.block_addresses.len();
        for _ in 0..padding_count {
            buf.extend_from_slice(&0u32.to_be_bytes());
        }

        // TOC
        // TOC entries are always a small fixed set; truncation is safe.
        #[allow(clippy::cast_possible_truncation)]
        let toc_count = self.toc.len() as u32;
        buf.extend_from_slice(&toc_count.to_be_bytes());
        for (name, value) in &self.toc {
            let name_bytes = name.as_bytes();
            // TOC key names (e.g., "DSDB") are always short ASCII; truncation is safe.
            #[allow(clippy::cast_possible_truncation)]
            buf.push(name_bytes.len() as u8);
            buf.extend_from_slice(name_bytes);
            buf.extend_from_slice(&value.to_be_bytes());
        }

        // Free list: 32 entries, each with count 0
        for _ in 0..32 {
            buf.extend_from_slice(&0u32.to_be_bytes());
        }

        buf
    }
}

// ---------------------------------------------------------------------------
// Decode
// ---------------------------------------------------------------------------

impl BinaryDecode for Bud1Prelude {
    /// Decode from at least 32 bytes, verifying the "Bud1" magic.
    fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < 32 {
            return Err(DecodeError::TooShort {
                expected: 32,
                got: data.len(),
            });
        }
        if &data[0..4] != b"Bud1" {
            return Err(DecodeError::InvalidMagic {
                expected: b"Bud1",
                got: data[0..4].to_vec(),
            });
        }
        let info_offset = u32::from_be_bytes(data[4..8].try_into().unwrap());
        let info_alloc = u32::from_be_bytes(data[8..12].try_into().unwrap());
        // bytes 12..16 are a duplicate of info_offset — skip
        let leaf_addr = u32::from_be_bytes(data[16..20].try_into().unwrap());
        // bytes 20..32 are reserved zeros — skip
        Ok(Self {
            info_offset,
            info_alloc,
            leaf_addr,
        })
    }
}

impl BinaryDecode for Dsdb {
    /// Decode from at least 20 bytes.
    fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < 20 {
            return Err(DecodeError::TooShort {
                expected: 20,
                got: data.len(),
            });
        }
        let root_node = u32::from_be_bytes(data[0..4].try_into().unwrap());
        // bytes 4..8: num_internal_nodes (skip)
        let num_records = u32::from_be_bytes(data[8..12].try_into().unwrap());
        // bytes 12..16: num_nodes (skip)
        // bytes 16..20: page_size (skip)
        Ok(Self {
            root_node,
            num_records,
        })
    }
}

impl BinaryDecode for AllocatorInfo {
    /// Decode allocator info: `num_offsets`, block address array (256 slots), TOC.
    fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        // Minimum: 4 (num_offsets) + 4 (reserved) + 256*4 (addrs) + 4 (toc_count) = 1036
        if data.len() < 1036 {
            return Err(DecodeError::TooShort {
                expected: 1036,
                got: data.len(),
            });
        }

        let num_offsets = u32::from_be_bytes(data[0..4].try_into().unwrap()) as usize;
        // bytes 4..8: reserved zero — skip

        let offsets_start = 8;
        let mut block_addresses = Vec::with_capacity(num_offsets);
        for i in 0..num_offsets {
            let base = offsets_start + i * 4;
            let addr = u32::from_be_bytes(data[base..base + 4].try_into().unwrap());
            block_addresses.push(addr);
        }

        // TOC starts after the 256-entry address array
        let toc_pos = offsets_start + 256 * 4;
        if data.len() < toc_pos + 4 {
            return Err(DecodeError::TooShort {
                expected: toc_pos + 4,
                got: data.len(),
            });
        }
        let toc_count = u32::from_be_bytes(data[toc_pos..toc_pos + 4].try_into().unwrap()) as usize;

        let mut toc = Vec::with_capacity(toc_count);
        let mut pos = toc_pos + 4;
        for _ in 0..toc_count {
            if pos >= data.len() {
                return Err(DecodeError::TooShort {
                    expected: pos + 1,
                    got: data.len(),
                });
            }
            let key_len = data[pos] as usize;
            pos += 1;
            if pos + key_len + 4 > data.len() {
                return Err(DecodeError::TooShort {
                    expected: pos + key_len + 4,
                    got: data.len(),
                });
            }
            let key = String::from_utf8_lossy(&data[pos..pos + key_len]).into_owned();
            pos += key_len;
            let value = u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap());
            pos += 4;
            toc.push((key, value));
        }

        Ok(Self {
            block_addresses,
            toc,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Encode tests ---

    #[test]
    fn bud1_prelude_encodes_to_32_bytes() {
        let prelude = Bud1Prelude {
            info_offset: 2080,
            info_alloc: 2048,
            leaf_addr: 0x100c,
        };
        let bytes = prelude.encode();
        assert_eq!(bytes.len(), 32);
        assert_eq!(&bytes[0..4], b"Bud1");
        assert_eq!(&bytes[4..8], &bytes[12..16]); // info_offset duplicated
    }

    #[test]
    fn dsdb_encodes_to_20_bytes() {
        let dsdb = Dsdb {
            root_node: 2,
            num_records: 6,
        };
        let bytes = dsdb.encode();
        assert_eq!(bytes.len(), 20);
        assert_eq!(u32::from_be_bytes(bytes[0..4].try_into().unwrap()), 2);
        assert_eq!(u32::from_be_bytes(bytes[8..12].try_into().unwrap()), 6);
        assert_eq!(
            u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
            0x1000
        );
    }

    #[test]
    fn dsdb_fixed_fields() {
        let dsdb = Dsdb {
            root_node: 0,
            num_records: 0,
        };
        let bytes = dsdb.encode();
        // num_internal_nodes = 0
        assert_eq!(u32::from_be_bytes(bytes[4..8].try_into().unwrap()), 0);
        // num_nodes = 1
        assert_eq!(u32::from_be_bytes(bytes[12..16].try_into().unwrap()), 1);
        // page_size = 0x1000
        assert_eq!(
            u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
            0x1000
        );
    }

    #[test]
    fn allocator_info_self_references() {
        let info = AllocatorInfo {
            block_addresses: vec![0x0820_000b, 0x0020_0005, 0x100c],
            toc: vec![("DSDB".to_string(), 1)],
        };
        let bytes = info.encode();
        let num = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        assert_eq!(num, 3);
        let b0 = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
        assert_eq!(b0, 0x0820_000b);
    }

    #[test]
    fn allocator_info_address_array_padded_to_256() {
        let info = AllocatorInfo {
            block_addresses: vec![0xAA],
            toc: vec![],
        };
        let bytes = info.encode();
        // 4 (num) + 4 (reserved) + 256*4 (addresses) = 1032
        // Then toc_count(4) = 1036
        // Then free list 32*4 = 128 => total 1164
        assert_eq!(bytes.len(), 1032 + 4 + 128);

        // First address is 0xAA, rest are zero
        let a0 = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
        assert_eq!(a0, 0xAA);
        // Entry 255 should be zero
        let a255 = u32::from_be_bytes(bytes[8 + 255 * 4..8 + 256 * 4].try_into().unwrap());
        assert_eq!(a255, 0);
    }

    #[test]
    fn allocator_info_toc_encoding() {
        let info = AllocatorInfo {
            block_addresses: vec![],
            toc: vec![("DSDB".to_string(), 1), ("AB".to_string(), 42)],
        };
        let bytes = info.encode();
        // TOC starts at offset 8 + 256*4 = 1032
        let toc_start = 1032;
        let toc_count = u32::from_be_bytes(bytes[toc_start..toc_start + 4].try_into().unwrap());
        assert_eq!(toc_count, 2);

        // First entry: len=4, "DSDB", value=1
        let mut pos = toc_start + 4;
        assert_eq!(bytes[pos], 4);
        pos += 1;
        assert_eq!(&bytes[pos..pos + 4], b"DSDB");
        pos += 4;
        assert_eq!(
            u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()),
            1
        );
        pos += 4;

        // Second entry: len=2, "AB", value=42
        assert_eq!(bytes[pos], 2);
        pos += 1;
        assert_eq!(&bytes[pos..pos + 2], b"AB");
        pos += 2;
        assert_eq!(
            u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()),
            42
        );
    }

    #[test]
    fn allocator_info_free_list_is_128_zero_bytes() {
        let info = AllocatorInfo {
            block_addresses: vec![],
            toc: vec![],
        };
        let bytes = info.encode();
        // Free list is the last 128 bytes
        let free_list = &bytes[bytes.len() - 128..];
        assert!(free_list.iter().all(|&b| b == 0));
    }

    // --- Helper function tests ---

    #[test]
    fn block_address_encoding() {
        let addr = block_address(32, 5);
        assert_eq!(addr & !0x1f, 32);
        assert_eq!(addr & 0x1f, 5);
    }

    #[test]
    fn block_address_larger_offset() {
        let addr = block_address(2080, 11);
        assert_eq!(addr & !0x1f, 2080);
        assert_eq!(addr & 0x1f, 11);
    }

    #[test]
    fn next_power_of_two_basics() {
        assert_eq!(next_power_of_two(1), 32);
        assert_eq!(next_power_of_two(20), 32);
        assert_eq!(next_power_of_two(32), 32);
        assert_eq!(next_power_of_two(33), 64);
        assert_eq!(next_power_of_two(1000), 1024);
    }

    #[test]
    fn log2_powers_of_two() {
        assert_eq!(log2(1), 0);
        assert_eq!(log2(2), 1);
        assert_eq!(log2(32), 5);
        assert_eq!(log2(1024), 10);
        assert_eq!(log2(2048), 11);
    }

    // --- Decode tests ---

    #[test]
    fn bud1_prelude_decode_rejects_short_data() {
        let result = Bud1Prelude::decode(&[0u8; 16]);
        assert!(result.is_err());
    }

    #[test]
    fn bud1_prelude_decode_rejects_bad_magic() {
        let mut data = [0u8; 32];
        data[0..4].copy_from_slice(b"Bad1");
        let result = Bud1Prelude::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn bud1_prelude_roundtrip() {
        let original = Bud1Prelude {
            info_offset: 2080,
            info_alloc: 2048,
            leaf_addr: 0x100c,
        };
        let bytes = original.encode();
        let decoded = Bud1Prelude::decode(&bytes).unwrap();
        assert_eq!(decoded.info_offset, original.info_offset);
        assert_eq!(decoded.info_alloc, original.info_alloc);
        assert_eq!(decoded.leaf_addr, original.leaf_addr);
    }

    #[test]
    fn dsdb_decode_rejects_short_data() {
        let result = Dsdb::decode(&[0u8; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn dsdb_roundtrip() {
        let original = Dsdb {
            root_node: 2,
            num_records: 6,
        };
        let bytes = original.encode();
        let decoded = Dsdb::decode(&bytes).unwrap();
        assert_eq!(decoded.root_node, original.root_node);
        assert_eq!(decoded.num_records, original.num_records);
    }

    #[test]
    fn allocator_info_decode_rejects_short_data() {
        let result = AllocatorInfo::decode(&[0u8; 100]);
        assert!(result.is_err());
    }

    #[test]
    fn allocator_info_roundtrip() {
        let original = AllocatorInfo {
            block_addresses: vec![0x0820_000b, 0x0020_0005, 0x100c],
            toc: vec![("DSDB".to_string(), 1)],
        };
        let bytes = original.encode();
        let decoded = AllocatorInfo::decode(&bytes).unwrap();
        assert_eq!(decoded.block_addresses, original.block_addresses);
        assert_eq!(decoded.toc, original.toc);
    }

    #[test]
    fn allocator_info_roundtrip_empty() {
        let original = AllocatorInfo {
            block_addresses: vec![],
            toc: vec![],
        };
        let bytes = original.encode();
        let decoded = AllocatorInfo::decode(&bytes).unwrap();
        assert_eq!(decoded.block_addresses, original.block_addresses);
        assert_eq!(decoded.toc, original.toc);
    }

    #[test]
    fn allocator_info_roundtrip_multi_toc() {
        let original = AllocatorInfo {
            block_addresses: vec![0x1234],
            toc: vec![("DSDB".to_string(), 1), ("free".to_string(), 99)],
        };
        let bytes = original.encode();
        let decoded = AllocatorInfo::decode(&bytes).unwrap();
        assert_eq!(decoded.block_addresses, original.block_addresses);
        assert_eq!(decoded.toc, original.toc);
    }
}
