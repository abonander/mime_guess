use uncased::Uncased;

include!("mime_types.rs");
include!(env!("MIME_TYPES_GENERATED_PATH"));

#[cfg(feature = "rev-mappings")]
#[derive(Copy, Clone)]
struct TopLevelExts {
    start: usize,
    end: usize,
    subs: &'static [(Uncased<'static>, (usize, usize))],
}

pub fn get_mime_types(ext: &str) -> Option<&'static [&'static str]> {
    MIME_TYPES
        .binary_search_by_key(&Uncased::new(ext), |(k, _)| (*k).into())
        .ok()
        .map(|i| MIME_TYPES[i].1)
}

#[cfg(feature = "rev-mappings")]
pub fn get_extensions(toplevel: &str, sublevel: &str) -> Option<&'static [&'static str]> {
    if toplevel == "*" {
        return Some(EXTS);
    }

    let top = map_lookup_top(REV_MAPPINGS, toplevel)?;

    if sublevel == "*" {
        return Some(&EXTS[top.start..top.end]);
    }

    let sub = map_lookup_sub(&top.subs, sublevel)?;
    Some(&EXTS[sub.0..sub.1])
}

fn map_lookup_top(map: &[(Uncased<'_>, TopLevelExts)], key: &str) -> Option<TopLevelExts> {
    map.binary_search_by_key(&Uncased::new(key), |(k, _)| k.clone())
        .ok()
        .map(|i| map[i].1)
}

fn map_lookup_sub(map: &[(Uncased<'_>, (usize, usize))], key: &str) -> Option<(usize, usize)> {
    map.binary_search_by_key(&Uncased::new(key), |(k, _)| k.clone())
        .ok()
        .map(|i| map[i].1)
}
