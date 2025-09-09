#[cfg(feature = "phf")]
extern crate phf_codegen;
extern crate uncased;

use uncased::Uncased;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::path::{self, Path};

use std::collections::BTreeMap;

use mime_types::MIME_TYPES;

#[path = "src/mime_types.rs"]
mod mime_types;

#[cfg(feature = "phf")]
const PHF_PATH: &str = "::impl_::phf";

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mime_types_generated_filename = "mime_types_generated.rs";
    let dest_path = Path::new(&out_dir).join(mime_types_generated_filename);
    let mut outfile = BufWriter::new(File::create(&dest_path).unwrap());

    println!(
        "cargo:rustc-env=MIME_TYPES_GENERATED_PATH={separator}{filename}",
        separator = path::MAIN_SEPARATOR,
        filename = mime_types_generated_filename,
    );

    #[cfg(feature = "phf")]
    build_forward_map(&mut outfile);

    #[cfg(feature = "rev-mappings")]
    build_rev_map(&mut outfile);
}

// Build forward mappings (ext -> mime type)
#[cfg(feature = "phf")]
fn build_forward_map<W: Write>(out: &mut W) {
    use phf_codegen::Map as PhfMap;

    let mut forward_map = PhfMap::new();
    forward_map.phf_path(PHF_PATH);

    let mut map_entries: Vec<(&str, Vec<&str>)> = Vec::new();

    for &(key, types) in MIME_TYPES {
        if let Some(&mut (key_, ref mut values)) = map_entries.last_mut() {
            // deduplicate extensions
            if key == key_ {
                values.extend_from_slice(types);
                continue;
            }
        }

        map_entries.push((key, types.into()));
    }

    for (key, values) in map_entries {
        forward_map.entry(Uncased::new(key), &format!("&{:?}", values));
    }

    writeln!(
        out,
        "static MIME_TYPES: phf::Map<Uncased<'static>, &'static [&'static str]> = \n{};",
        forward_map.build()
    )
    .unwrap();
}

// Build reverse mappings (mime type -> ext)
#[cfg(all(feature = "phf", feature = "rev-mappings"))]
fn build_rev_map<W: Write>(out: &mut W) {
    use phf_codegen::Map as PhfMap;

    let dyn_map = get_rev_mappings();

    let mut rev_map = PhfMap::new();
    rev_map.phf_path(PHF_PATH);

    let mut exts = Vec::new();

    for (top, subs) in dyn_map {
        let top_start = exts.len();

        let mut sub_map = PhfMap::new();
        sub_map.phf_path(PHF_PATH);

        for (sub, sub_exts) in subs {
            let sub_start = exts.len();
            exts.extend(sub_exts);
            let sub_end = exts.len();

            sub_map.entry(sub, &format!("({}, {})", sub_start, sub_end));
        }

        let top_end = exts.len();

        rev_map.entry(
            top,
            &format!(
                "TopLevelExts {{ start: {}, end: {}, subs: {} }}",
                top_start,
                top_end,
                sub_map.build()
            ),
        );
    }

    writeln!(
        out,
        "static REV_MAPPINGS: phf::Map<Uncased<'static>, TopLevelExts> = \n{};",
        rev_map.build()
    )
    .unwrap();

    writeln!(out, "const EXTS: &'static [&'static str] = &{:?};", exts).unwrap();
}

#[cfg(all(not(feature = "phf"), feature = "rev-mappings"))]
fn build_rev_map<W: Write>(out: &mut W) {
    use std::fmt::Write as _;

    let dyn_map = get_rev_mappings();

    write!(
        out,
        "static REV_MAPPINGS: &'static [(Uncased<'static>, TopLevelExts)] = &["
    )
    .unwrap();

    let mut exts = Vec::new();

    for (top, subs) in dyn_map {
        let top_start = exts.len();

        let mut sub_map = String::new();

        for (sub, sub_exts) in subs {
            let sub_start = exts.len();
            exts.extend(sub_exts);
            let sub_end = exts.len();

            write!(
                sub_map,
                "(Uncased::from_borrowed(\"{}\"), ({}, {})),",
                sub, sub_start, sub_end
            )
            .unwrap();
        }

        let top_end = exts.len();

        write!(
            out,
            "(Uncased::from_borrowed(\"{}\"), TopLevelExts {{ start: {}, end: {}, subs: &[{}] }}),",
            top, top_start, top_end, sub_map
        )
        .unwrap();
    }

    writeln!(out, "];").unwrap();

    writeln!(out, "const EXTS: &'static [&'static str] = &{:?};", exts).unwrap();
}

#[cfg(feature = "rev-mappings")]
fn get_rev_mappings() -> BTreeMap<Uncased<'static>, BTreeMap<Uncased<'static>, Vec<&'static str>>> {
    // First, collect all the mime type -> ext mappings)
    let mut dyn_map = BTreeMap::new();
    for &(key, types) in MIME_TYPES {
        for val in types {
            let (top, sub) = split_mime(val);
            dyn_map
                .entry(Uncased::new(top))
                .or_insert_with(BTreeMap::new)
                .entry(Uncased::new(sub))
                .or_insert_with(Vec::new)
                .push(key);
        }
    }
    dyn_map
}

fn split_mime(mime: &str) -> (&str, &str) {
    let split_idx = mime.find('/').unwrap();
    (&mime[..split_idx], &mime[split_idx + 1..])
}
