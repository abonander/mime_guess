use std::{ptr};
use std::borrow::Cow;
use std::convert::TryInto;
use std::fmt::{Debug, Formatter};
use std::str;

use memchr::{memchr, memmem};

#[cfg(not(without_generated_data))]
include!("generated/data.rs");

#[cfg(not(without_generated_data))]
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

    /// Byte indices into `packed_extensions`,
    /// where `idx[n]` and `idx[n + 1]` are the start and end of each extension, respectively.
    pub extension_offsets: &'a [PackedIdx],
    pub packed_extensions: &'a str,

    /// Similar to `extension_offsets`, but where `idx[n]` and `idx[n + 1]` are the start and end
    /// indices into `packed_extension_mimes`.
    pub extension_mimes_offsets: &'a [PackedIdx],

    /// Indices into `mime_offsets`.
    pub extension_mimes: &'a [PackedIdx],

    /// Same as `extension_offsets`, but for `packed_mimes`.
    pub mime_offsets: &'a [PackedIdx],
    pub packed_mimes: &'a str,
}

#[cfg(without_generated_data)]
pub const PACKED_DATA: PackedData<'static> = PackedData {
    extension_lut: &[],
    lut_offset: 0,
    extension_offsets: &[],
    packed_extensions: "",
    extension_mimes_offsets: &[],
    extension_mimes: &[],
    mime_offsets: &[],
    packed_mimes: "",
};

#[derive(Copy, Clone)]
pub struct Mimes<'data> {
    extension_mimes: &'data [PackedIdx],
    mime_offsets: &'data [PackedIdx],
    packed_mimes: &'data str,
}

pub fn get_mimes<'data>(
    data: &PackedData<'data>,
    ext: &str,
) -> Option<Mimes<'data>> {
    // Guarantees `ext` is not empty and its first character is ASCII.
    let lut_idx = extension_lut_index(data.lut_offset, ext)? as usize;

    let extension_offsets_range = get_packed_range(&data.extension_lut, lut_idx)?;

    for i in extension_offsets_range {
        let extension_range = get_packed_range(&data.extension_offsets, i)?;

        if !data.packed_extensions
            // Slicing as bytes skips the `is_char_boundary()` check which ends up being
            // surprisingly expensive.
            //
            // `str::eq_ignore_ascii_case()` just forwards to the bytes impl anyway.
            .as_bytes()
            .get(extension_range)?
            .eq_ignore_ascii_case(ext.as_bytes())
        {
            continue;
        }

        let mimes_offsets = get_packed_range(&data.extension_mimes_offsets, i)?;
        let extension_mimes = data.extension_mimes.get(mimes_offsets)?;

        return Some(Mimes {
            extension_mimes,
            mime_offsets: &data.mime_offsets,
            packed_mimes: &data.packed_mimes,
        })
    }

    None
}

impl<'a> Mimes<'a> {
    pub const EMPTY: Mimes<'static> = Mimes {
        extension_mimes: &[],
        mime_offsets: &[],
        packed_mimes: "",
    };

    pub fn len(&self) -> usize {
        self.extension_mimes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.extension_mimes.is_empty()
    }

    pub fn get(&self, idx: usize) -> Option<&'a str> {
        let idx: usize = (*self.extension_mimes.get(idx)?).into();

        let range = get_packed_range(&self.mime_offsets, idx)?;

        self.packed_mimes.get(range)
    }
}

impl<'a> PartialEq for Mimes<'a> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.packed_mimes, other.packed_mimes)
            && ptr::eq(self.mime_offsets, other.mime_offsets)
            && self.extension_mimes == other.extension_mimes
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
    let c: u8 = ext
        .chars()
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

fn get_packed_range(indices: &[PackedIdx], i: usize) -> Option<std::ops::Range<usize>> {
    let start: usize = (*indices.get(i)?).into();
    let end: usize = (*indices.get(i.checked_add(1)?)?).into();

    Some(start .. end)
}

#[cfg(test)]
mod test {
    #[test]
    fn test_specific_exts() {
        let exts = ["z1"];

        for ext in exts {
            let resolved_mimes = super::get_mimes(&super::PACKED_DATA, ext)
                .unwrap_or_else(|| panic!("no resolved mimes for {ext:?}"));
        }
    }

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
