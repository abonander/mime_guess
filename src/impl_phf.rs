extern crate phf;

use unicase::UniCase;

include!(concat!(env!("OUT_DIR"), "/mime_types_generated.rs"));

#[cfg(feature = "rev-mappings")]
struct TopLevelExts {
    start: usize,
    end: usize,
    subs: phf::Map<UniCase<&'static str>, (usize, usize)>,
}

pub fn get_mime_types(ext: &str) -> Option<&'static [&'static str]> {
    map_lookup(&MIME_TYPES, ext)
}

pub fn get_extensions(toplevel: &str, sublevel: &str) -> Option<&'static [&'static str]> {
    if toplevel == "*" {
        return Some(EXTS);
    }

    let top = try_opt!(map_lookup(&REV_MAPPINGS, toplevel));

    if sublevel == "*" {
        return Some(&EXTS[top.start..top.end]);
    }

    let sub = try_opt!(map_lookup(&top.subs, sublevel));
    Some(&EXTS[sub.0..sub.1])
}

fn map_lookup<'map, V>(
    map: &'map phf::Map<UniCase<&'static str>, V>,
    key: &str,
) -> Option<&'map V> {
    // This transmute should be safe as `get` will not store the reference with
    // the expanded lifetime. This is due to `Borrow` being overly strict and
    // can't have an impl for `&'static str` to `Borrow<&'a str>`.
    //
    // See https://github.com/rust-lang/rust/issues/28853#issuecomment-158735548
    let key = unsafe { ::std::mem::transmute::<&str, &'static str>(key) };
    map.get(&UniCase::new(key))
}