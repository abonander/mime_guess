extern crate phf_codegen;

use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;

const GENERATED_FILE: &'static str = "src/mime_types_generated.rs";

mod mime_types;

fn main() {
    let mut outfile = BufWriter::new(File::create(GENERATED_FILE).unwrap());

    write!(outfile, "static MIME_TYPES: phf::Map<&'static str, &'static str> = ").unwrap();

    let mut map = phf_codegen::Map::new();

    for &(key, val) in mime_types::MIME_TYPES {
        map.entry(key, &format!("{:?}", val));
    }

    map.build(&mut outfile).unwrap();

    writeln!(outfile, ";").unwrap();
}
