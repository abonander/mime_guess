use std::{env, fs};
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::rc::Rc;
use mime_guess::lut;

#[derive(serde::Deserialize)]
struct MimeData {
    #[serde(default)]
    extensions: Vec<Rc<str>>,

    #[serde(skip)]
    index: Option<PackedIdx>
}

use mime_guess::lut::{PackedIdx, GUARD_END, GUARD_START};


fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);

    let Some(release_tag) = args.next() else {
        println!(
            "usage: build-lut <release-tag>\n
            \n\
             where <release-tag> is a valid tag from https://github.com/jshttp/mime-db/tags"
        );

        return Ok(());
    };

    let url = format!("https://cdn.jsdelivr.net/gh/jshttp/mime-db@{release_tag}/db.json");

    let mut mimedb = reqwest::blocking::get(&url)?
        .error_for_status()?
        .json::<BTreeMap<Rc<str>, MimeData>>()?;

    // Generate `mimes_data`: `GUARD_START <mime> GUARD_END..`
    let mut mime_offsets = Vec::<PackedIdx>::with_capacity(mimedb.len());
    let mut packed_mimes = String::with_capacity(PackedIdx::MAX.into());

    assert!(!mimedb.is_empty());

    mime_offsets.push(0);

    for (mime, data) in &mut mimedb {
        // A lot of media-types in the database don't have any nominal extensions.
        // We don't care about those.
        if data.extensions.is_empty() {
            continue;
        }

        packed_mimes.push_str(&mime);

        data.index = Some((mime_offsets.len() - 1).try_into().expect("`mime_offsets.len() -1` exceeds `PackedIdx::MAX`"));

        mime_offsets.push(packed_mimes.len().try_into().expect("`packed_mimes.len()` `exceeds `PackedIdx::MAX`"));
    }

    let mut extensions_to_mimes: BTreeMap<Rc<str>, Vec<Rc<str>>> = BTreeMap::new();

    for (mime, data) in &mimedb {
        for ext in &data.extensions {
            extensions_to_mimes.entry(ext.clone())
                .or_default()
                .push(mime.clone());
        }
    }

    // Build `extension_lut`, `extension_data` and `extension_mimes`
    let first_ext = extensions_to_mimes
        .keys()
        .next()
        .expect("extensions_to_mimes cannot be empty");

    let first_lut_index = lut::extension_lut_index(0, first_ext)
        .expect("first extension is empty or is not ASCII");

    // We can save a ton of space in the LUT by making the first character seen the zero index.
    //
    // Anything before that cannot be in the LUT, so we can just offset by that amount,
    // as long as we use checked arithmetic to avoid underflow.
    let lut_offset = first_lut_index;

    let mut extension_lut = Vec::with_capacity(128);

    let mut extension_offsets = Vec::<PackedIdx>::with_capacity(extensions_to_mimes.len());

    let mut packed_extensions = String::with_capacity(PackedIdx::MAX.into());

    let mut extension_mimes_offsets = Vec::<PackedIdx>::with_capacity(extensions_to_mimes.len());

    let mut extension_mimes = Vec::<PackedIdx>::with_capacity(PackedIdx::MAX.into());

    // The first entries of all tables _must_ be zero.
    extension_lut.push(0);
    extension_offsets.push(0);
    extension_mimes_offsets.push(0);

    let mut last_lut_index: usize = 0;

    for (i, (ext, mimes)) in extensions_to_mimes.iter().enumerate() {
        let lut_index: usize = lut::extension_lut_index(lut_offset, ext)
            .unwrap_or_else(|| panic!("extension {i} ({ext:?}) could not be converted to a LUT index"))
            .into();

        // Push a new LUT entry if it's a new index.
        if lut_index != last_lut_index {
            let offset_idx: PackedIdx = (extension_offsets.len() - 1)
                .try_into()
                .unwrap_or_else(|_| panic!("extension {i} ({ext:?}) offset index overflows `PackedIdx`"));

            // Fill skipped entries in the LUT with copies of the next index.
            //
            // This way we don't have to search more than one index ahead to know how long
            // the extension data for a given LUT entry is.
            //
            // Empty entries will see a zero-length range.
            extension_lut.resize(usize::from(lut_index) + 1, offset_idx);
        }

        last_lut_index = lut_index;

        // Push the extension and the next (or last) offset.
        packed_extensions.push_str(ext);
        extension_offsets.push(
            packed_extensions.len().try_into()
                .unwrap_or_else(|_| panic!("extension {i} ({ext:?}) offset overflows `PackedIdx`"))
        );

        // Push the extension's mime indices
        for mime in mimes {
            let mime_data = mimedb.get(mime)
                .unwrap_or_else(|| panic!("BUG: mime {mime:?} not added to `mimedb`"));

            let mime_index = mime_data.index
                .unwrap_or_else(|| panic!("BUG: mime {mime:?} not assigned an index"));

            extension_mimes.push(mime_index);
        }

        // Push the next (or last) offset
        extension_mimes_offsets.push(
            extension_mimes
                .len()
                .try_into()
                .unwrap_or_else(|_| panic!("extension {i} ({ext:?}) mime offset overflowed `PackedIdx`"))
        );
    }

    // Since we don't always push a LUT entry, we need to push the last one.
    extension_lut.push(extension_offsets.len().try_into().expect("extension_offsets.len() overflows `PackedIdx`"));

    let mut data_rs = BufWriter::new(File::create("src/lut/generated/data.rs")?);

    // Note: will break with `-Z fmt-debug=none`
    writeln!(data_rs, "\
pub const PACKED_DATA: PackedData<'static> = PackedData {{
    extension_lut: &{extension_lut:?},
    lut_offset: {lut_offset:?},

    extension_offsets: &{extension_offsets:?},
    packed_extensions: {packed_extensions:?},

    extension_mimes_offsets: &{extension_mimes_offsets:?},
    extension_mimes: &{extension_mimes:?},

    mime_offsets: &{mime_offsets:?},
    packed_mimes: {packed_mimes:?},
}};")?;

    data_rs.into_inner()?.sync_data()?;

    let mut test_data = BufWriter::new(File::create("src/lut/generated/test_data.rs")?);

    writeln!(
        test_data,
        "pub const EXT_TO_MIME: &'static [(&'static str, &'static [&'static str])] = &["
    )?;

    for (ext, mimes) in &extensions_to_mimes {
        write!(
            test_data,
            "    (\n        \"{}\",\n        &[",
            ext.escape_debug(),
        )?;

        for mime in mimes {
            write!(test_data, "\"{}\",", mime.escape_debug())?;
        }

        writeln!(test_data, "],\n    ),")?;
    }

    writeln!(test_data, "];")?;

    Ok(())
}
