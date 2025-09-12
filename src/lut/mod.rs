use std::{ptr};
use std::borrow::Cow;
use std::convert::TryInto;
use std::fmt::{Debug, Formatter};
use std::str;

use memchr::{memchr, memmem};

include!("generated/data.rs");

#[cfg(test)]
include!("generated/test_data.rs");

/// The smallest type that all indices fit into.
pub type PackedIdx = u16;

pub const PACKED_IDX_SIZE: usize = size_of::<PackedIdx>();

// 0xF5 - 0xFF are not allowed to appear in a UTF-8 string, so they make perfect guard bytes.
//
// The start and end bytes are different to ensure we cannot mistake the end of one string
// for the packed index + guard byte of the start of the next.
//
// We reserve `0xFF..` as a sentinel value for `PackedIdx`.
pub const GUARD_START: u8 = 0xFD;
pub const GUARD_END: u8 = 0xFE;

pub struct PackedData<'a> {
    /// Lookup table into `extension_data`, indexed by `extension[0] - lut_offset`.
    pub extension_lut: &'a [PackedIdx],
    pub lut_offset: u8,

    pub extension_data: &'a [u8],

    pub extension_mimes: &'a [u8],

    pub mime_data: &'a [u8],
}

#[derive(Copy, Clone)]
pub struct Mimes<'data> {
    mimes_indices: &'data [u8],
    mimes_data: &'data [u8],
}

pub fn get_mimes<'data>(
    data: &PackedData<'data>,
    ext: &str,
) -> Option<Mimes<'data>> {
    // Guarantees `ext` is not empty and its first character is ASCII.
    let index = extension_lut_index(data.lut_offset, ext)?;

    let search_start = *data.extension_lut.get(index as usize)? as usize;
    let search_end = data.extension_lut.get(index as usize + 1)
        .map(|i| *i as usize)
        .unwrap_or(data.extension_data.len());

    let extension_data = &data.extension_data[search_start .. search_end];

    let mut scratch = [GUARD_START; 16];
    let needle = make_ext_needle(ext, &mut scratch);

    let needle_idx = memmem::find(extension_data, &needle)?;

    let mimes_idx: usize = extension_data[needle_idx + needle.len()..][..PACKED_IDX_SIZE]
        .try_into()
        .map(PackedIdx::from_le_bytes)
        .ok()?
        .into();

    // Length byte, then
    let mimes_len: usize = (*data.extension_mimes.get(mimes_idx)?).into();

    let mimes_indices = data.extension_mimes.get(mimes_idx.checked_add(1)?..)?
        .get(..mimes_len.checked_mul(PACKED_IDX_SIZE)?)?;

    Some(Mimes {
        mimes_indices,
        mimes_data: data.mime_data,
    })
}

impl<'a> Mimes<'a> {
    pub const EMPTY: Mimes<'static> = Mimes {
        mimes_indices: &[],
        mimes_data: &[]
    };

    pub fn len(&self) -> usize {
        self.mimes_indices.len() / PACKED_IDX_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.mimes_indices.is_empty()
    }

    pub fn get(&self, idx: usize) -> Option<&'a str> {
        let idx: usize = self.mimes_indices
            .get(idx.checked_mul(PACKED_IDX_SIZE)?..)?
            .get(..PACKED_IDX_SIZE)?
            .try_into()
            .map(PackedIdx::from_le_bytes)
            .ok()?
            .into();

        let (guard_start, mime) = self.mimes_data.get(idx..)?.split_first()?;

        assert_eq!(*guard_start, GUARD_START, "index {idx} in data not a guard byte");

        let mime_end = memchr(GUARD_END, mime)
            .unwrap_or_else(|| panic!("no guard byte after {idx}"));

        Some(str::from_utf8(&mime[..mime_end]).unwrap_or_else(|e| panic!("mime {} is not valid UTF-8: {e}", mime.escape_ascii())))
    }
}

impl<'a> PartialEq for Mimes<'a> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.mimes_data, other.mimes_data)
            && self.mimes_indices == other.mimes_indices
    }
}

impl Eq for Mimes<'_> {}

impl Debug for Mimes<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries((0..self.len()).filter_map(|i| self.get(i)))
            .finish()
    }
}

pub fn extension_lut_index(offset: u8, ext: &str) -> Option<u8> {
    let c: u8 = ext.chars()
        .next()?
        // We fold to uppercase because it makes the resulting LUT smaller,
        // as lowercase ASCII characters have the highest byte values.
        .to_ascii_uppercase()
        .try_into()
        .ok()?;

    Some(c.checked_sub(offset)?)
}

fn make_ext_needle<'a>(ext: &str, scratch: &'a mut [u8]) -> Cow<'a, [u8]> {
    debug_assert_eq!(scratch[0], GUARD_START);
    debug_assert!(scratch.len() > 1);

    fn copy_ascii_lowercase(s: &str, out: &mut [u8]) {
        for (c, o) in s.as_bytes().iter().zip(out) {
            *o = c.to_ascii_lowercase();
        }
    }

    // It's actually quicker to just copy the extension over, because it eliminates branches.
    if ext.len() < scratch.len() {
        copy_ascii_lowercase(ext, &mut scratch[1..][..ext.len()]);
        scratch[1 + ext.len()] = GUARD_END;

        return scratch[..2 + ext.len()].into();
    }

    let mut needle = vec![GUARD_START; ext.len() + 2];

    copy_ascii_lowercase(ext, &mut needle[1..][..ext.len()]);
    needle[1 + ext.len()] = GUARD_END;

    needle.into()
}

#[cfg(test)]
mod test {
    #[test]
    fn test_all_exts() {
        for (ext, mimes) in super::EXT_TO_MIME {
            let resolved_mimes = super::get_mimes(&super::PACKED_DATA, ext)
                .unwrap_or_else(|| panic!("no resolved mimes for {ext:?}"));

            assert_eq!(resolved_mimes.len(), mimes.len());

            for (i, &mime) in mimes.iter().enumerate() {
                let resolved_mime = resolved_mimes.get(i);
                assert_eq!(resolved_mime, Some(mime), "ext {ext:?} missing resolved mime at {i}");
            }
        }
    }
}
