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
    data_idx: Option<PackedIdx>
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
    let mut mime_data = Vec::new();

    for (mime, data) in &mut mimedb {
        // A lot of media-types in the database don't have any nominal extensions.
        // We don't care about those.
        if data.extensions.is_empty() {
            continue;
        }

        data.data_idx = Some(
            mime_data
                .len()
                .try_into()
                .unwrap_or_else(|_| panic!("Mime {mime:?} offset exceeds max value for `PackedIdx`"))
        );

        mime_data.push(lut::GUARD_START);
        mime_data.extend_from_slice(mime.as_bytes());
        mime_data.push(lut::GUARD_END);
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
    let (first_ext, first_ext_mimes) = extensions_to_mimes
        .iter()
        .next()
        .expect("extensions_to_mimes cannot be empty");


    let first_lut_index = lut::extension_lut_index(0, first_ext)
        .expect("first extension is empty or is not ASCII");

    // We can save a ton of space in the LUT by making the first character seen the zero index.
    //
    // Anything before that cannot be in the LUT, so we can just offset by that amount,
    // as long as we use checked arithmetic to avoid underflow.
    let lut_offset = first_lut_index;

    // The first character in the LUT *must* be zero.
    //
    // To simplify the implementation, we still emit it in the look-up table.
    let mut extension_lut = vec![0];
    let mut extension_data = Vec::<u8>::new();
    let mut extension_mimes = Vec::new();

    write_extension_mime_entry(first_ext, first_ext_mimes, &mimedb, &mut extension_data, &mut extension_mimes);

    let mut last_lut_index = 0;
    let mut last_data_index = 0;

    for (i, (ext, mimes)) in extensions_to_mimes.iter().enumerate().skip(1) {
        let lut_index: u8 = lut::extension_lut_index(lut_offset, ext)
            .unwrap_or_else(|| panic!("extension {i} ({ext:?}) could not be converted to a LUT index"));

        // Only push a new LUT entry if it's a new index.
        if lut_index != last_lut_index {
            let data_idx: PackedIdx = extension_data.len()
                .try_into()
                .unwrap_or_else(|_| panic!("extension {i} ({ext:?}) data index overflows `PackedIdx`"));

            // Fill skipped entries in the LUT with the next index.
            //
            // This way we don't have to search more than one index ahead to know how long
            // the extension data for a given LUT entry is.
            //
            // Empty entries will see a zero-length range.
            extension_lut.extend((last_lut_index .. lut_index).map(|_| data_idx));
        }

        last_lut_index = lut_index;

        write_extension_mime_entry(ext, mimes, &mimedb, &mut extension_data, &mut extension_mimes);
    }

    fs::write("src/lut/generated/extension_data", &extension_data)?;
    fs::write("src/lut/generated/extension_mimes", &extension_mimes)?;
    fs::write("src/lut/generated/mime_data", &mime_data)?;

    let mut data_rs = BufWriter::new(File::create("src/lut/generated/data.rs")?);

    // Note: resolved paths are actually relative to the `include!()`ed file.
    writeln!(data_rs, "\
pub const PACKED_DATA: PackedData<'static> = PackedData {{
    lut_offset: {lut_offset},
    extension_data: include_bytes!(\"extension_data\"),
    extension_mimes: include_bytes!(\"extension_mimes\"),
    mime_data: include_bytes!(\"mime_data\"),
    extension_lut: &["
    )?;

    for data_idx in extension_lut {
        writeln!(data_rs, "    {data_idx},")?;
    }

    writeln!(data_rs, "    ],\n}};")?;

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

fn write_extension_mime_entry(
    ext: &str,
    mimes: &[Rc<str>],
    mimedb: &BTreeMap<Rc<str>, MimeData>,
    extension_data: &mut Vec<u8>,
    extension_mimes: &mut Vec<u8>,
) {
    extension_data.push(GUARD_START);
    extension_data.extend_from_slice(ext.as_bytes());
    extension_data.push(GUARD_END);

    let extension_mimes_idx: PackedIdx = extension_mimes
        .len()
        .try_into()
        .unwrap_or_else(|_| panic!("extension {ext:?} offset in extension_mimes overflows `PackedIdx"));

    extension_data.extend(extension_mimes_idx.to_le_bytes());

    let mimes_len: u8 = mimes
        .len()
        .try_into()
        .unwrap_or_else(|_| panic!("did not expect extension {ext:?} to have more than 256 Mimes"));

    extension_mimes.push(mimes_len);

    for mime in mimes {
        let mime_data = mimedb
            .get(mime)
            .unwrap_or_else(|| panic!("BUG: Mime {mime:?} missing from `mimedb`"));

        let data_idx = mime_data.data_idx
            .unwrap_or_else(|| panic!("BUG: Mime {mime:?} was not assigned a `data_idx`"));

        extension_mimes.extend(data_idx.to_le_bytes());
    }
}
