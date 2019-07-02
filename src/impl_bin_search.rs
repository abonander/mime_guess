use unicase::UniCase;
include!("mime_types.rs");

#[cfg(feature = "rev-mappings")]
struct TopLevelExts {
    start: usize,
    end: usize,
    subs: &'static [(UniCase<&'static str>, (usize, usize))],
}

pub fn get_mime_types(ext: &str) -> Option<&'static [&'static str]> {
    let ext = UniCase::new(ext);

    map_lookup(MIME_TYPES, &ext)
}

pub fn get_extensions(toplevel: &str, sublevel: &str) -> Option<&'static [&'static str]> {
    if toplevel == "*" {
        return Some(EXTS);
    }

    let top = map_lookup(REV_MAPPINGS, key)?;

    if sublevel == "*" {
        return Some(&EXTS[top.start..top.end]);
    }

    let sub = map_lookup(&top.subs, sublevel)?;
    Some(&EXTS[sub.0..sub.1])
}

fn map_lookup<V>(map: &'static [(&'static str, V)], key: &str) -> Option<V> {
    map.binary_search_by_key(&UniCase::new(key), |(k, _)| UniCase::ascii(k))
}