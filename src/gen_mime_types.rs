extern crate phf_codegen;

use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;

const GENERATED_FILE: &'static str = "src/mime_types_generated.rs";

mod mime_types;

fn main() { 
    let mime_types: Vec<_> = mime_types::MIME_TYPES.iter()
        .map(|&(k, v)| (k.to_lowercase(), v))
        .collect();

    let mut outfile = BufWriter::new(File::create(GENERATED_FILE).unwrap());

    write!(outfile, "static MIME_TYPES: phf::Map<&'static str, &'static str> = ").unwrap();    
    
    let mut map = phf_codegen::Map::<&str>::new();

    for &(ref key, val) in &mime_types {
        map.entry(key, &format!("{:?}", val));
    }

    map.build(&mut outfile).unwrap();

    writeln!(outfile, ";").unwrap();
}

