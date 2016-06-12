extern crate phf_codegen;
extern crate unicase;

use phf_codegen::Map as PhfMap;

use unicase::UniCase;

use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;

use std::collections::HashMap;

use mime_types::MIME_TYPES;

const GENERATED_FILE: &'static str = "src/mime_types_generated.rs";

mod mime_types;

fn main() {
   
    let mut outfile = BufWriter::new(File::create(GENERATED_FILE).unwrap());

    build_forward_map(&mut outfile);

    build_rev_map(&mut outfile);
}

// Build forward mappings (ext -> mime type)
fn build_forward_map<W: Write>(out: &mut W) {
    write!(out, "static MIME_TYPES: phf::Map<UniCase<&'static str>, &'static str> = ").unwrap();
    let mut forward_map = PhfMap::new();

    for &(key, val) in MIME_TYPES {
        forward_map.entry(UniCase(key), &format!("{:?}", val));
    }

    forward_map.build(out).unwrap();

    writeln!(out, ";").unwrap();
}

// Build reverse mappings (mime type -> ext)
fn build_rev_map<W: Write>(out: &mut W) {
    // First, collect all the mime type -> ext mappings)
    let mut dyn_map = HashMap::new();

    for &(key, val) in MIME_TYPES {
        dyn_map.entry(UniCase(val)).or_insert_with(Vec::new).push(key);
    }

    write!(out, "static REV_MAPPINGS: phf::Map<UniCase<&'static str>, &'static [&'static str]> = ").unwrap();

    let mut rev_map = PhfMap::new(); 

    for (key, vals) in dyn_map {
        // FIXME: ugly substitute to force coercion (type ascription doesn't work) 
        // Workaround unnecessary if this issue is fixed: https://github.com/rust-lang/rust/issues/31260
        rev_map.entry(key, &format!("{{const VALS: &'static [&'static str] = &{:?}; VALS}}", vals));        
    }

    rev_map.build(out).unwrap();
    writeln!(out, ";").unwrap();
}

