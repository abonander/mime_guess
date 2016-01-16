//! Guessing of MIME types by file extension.
//!
//! Uses a static list of file-extension : MIME type mappings.
#![cfg_attr(feature = "bench", feature(test))]

extern crate mime;
extern crate phf;

use mime::Mime;

use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::Path;

include!("mime_types_generated.rs");

/// Guess the MIME type of `path` by its extension (as defined by `Path::extension()`).
///
/// If `path` has no extension, or its extension has no known MIME type mapping,
/// then the MIME type is assumed to be `application/octet-stream`.
///
/// ##Note
/// **Guess** is the operative word here, as there are no guarantees that the contents of the file
/// that `path` points to match the MIME type associated with the path's extension.
///
/// Take care when processing files with assumptions based on the return value of this function.
pub fn guess_mime_type<P: AsRef<Path>>(path: P) -> Mime {
    guess_mime_type_opt(path)
        .unwrap_or_else(octet_stream)
}

/// Guess the MIME type of `path` by its extension (as defined by `Path::extension()`).
///
/// If `path` has no extension, or its extension has no known MIME type mapping,
/// then `None` returned.
///
/// ##Note
/// **Guess** is the operative word here, as there are no guarantees that the contents of the file
/// that `path` points to match the MIME type associated with the path's extension.
///
/// Take care when processing files with assumptions based on the return value of this function.
pub fn guess_mime_type_opt<P: AsRef<Path>>(path: P) -> Option<Mime> {
    let ext = path.as_ref().extension().and_then(OsStr::to_str).unwrap_or("");

    get_mime_type_opt(ext)
}

/// Get the MIME type associated with a file extension.
///
/// If there is no association for the extension, or `ext` is empty,
/// `application/octet-stream` is returned.
pub fn get_mime_type(search_ext: &str) -> Mime {
    get_mime_type_opt(search_ext)
        .unwrap_or_else(octet_stream)
}

/// Get the MIME type associated with a file extension.
///
/// If there is no association for the extension, or `ext` is empty,
/// `None` is returned.
pub fn get_mime_type_opt(search_ext: &str) -> Option<Mime> {
    get_mime_type_str(search_ext)
        .map(|mime| mime.parse::<Mime>().unwrap())
}

/// Get the MIME type string associated with a file extension. Case-insensitive.
///
/// If `search_ext` is not already lowercase,
/// it will be converted to lowercase to facilitate the search.
///
/// Returns `None` if `search_ext` is empty or an associated extension was not found.
pub fn get_mime_type_str(search_ext: &str) -> Option<&'static str> {
    if search_ext.is_empty() { return None; }

    let search_ext = to_lowercase_cow(search_ext);
    MIME_TYPES.get(&*search_ext).cloned()  
}

/// Get the MIME type for `application/octet-stream` (generic binary stream)
pub fn octet_stream() -> Mime {
    "application/octet-stream".parse().unwrap()
}

/// Convert `mixed` to lowercase if it is not already all lowercase.
fn to_lowercase_cow(mixed: &str) -> Cow<str> {
    use std::char;

    // This seems like an unnecessary optimization but on my machine
    // it cuts the average search time by a factor of 3 over unconditionally
    // calling .to_lowercase().
    if mixed.chars().any(char::is_uppercase) {
        mixed.to_lowercase().into()
    } else {
        mixed.into()
    }
}

#[cfg(test)]
mod tests {
    use mime::Mime;
    use std::ascii::AsciiExt;
    use std::path::Path;
    use super::{get_mime_type, guess_mime_type, MIME_TYPES};
    use super::{get_mime_type_opt, guess_mime_type_opt};

    #[test]
    fn test_mime_type_guessing() {
        assert_eq!(get_mime_type("gif").to_string(), "image/gif".to_string());
        assert_eq!(get_mime_type("TXT").to_string(), "text/plain".to_string());
        assert_eq!(get_mime_type("blahblah").to_string(), "application/octet-stream".to_string());

        assert_eq!(guess_mime_type(Path::new("/path/to/file.gif")).to_string(), "image/gif".to_string());
        assert_eq!(guess_mime_type("/path/to/file.gif").to_string(), "image/gif".to_string());
    }

    #[test]
    fn test_mime_type_guessing_opt() {
        assert_eq!(get_mime_type_opt("gif").unwrap().to_string(), "image/gif".to_string());
        assert_eq!(get_mime_type_opt("TXT").unwrap().to_string(), "text/plain".to_string());
        assert_eq!(get_mime_type_opt("blahblah"), None);

        assert_eq!(guess_mime_type_opt("/path/to/file.gif").unwrap().to_string(), "image/gif".to_string());
        assert_eq!(guess_mime_type_opt("/path/to/file"), None);
    }

    #[test]
    fn test_are_extensions_lowercase() {
        for (mime_ext, _) in &MIME_TYPES {
            assert!(
                mime_ext.chars().all(|ch| ch.to_ascii_lowercase() == ch), 
                "extension not lowercase: {}", mime_ext
            );
        }
    }

    #[test]
    fn test_are_mime_types_parseable() {
        for (_, mime) in &MIME_TYPES {
            mime.parse::<Mime>().unwrap();
        }
    }

    #[test]
    fn test_are_extensions_ascii() {
        for (ext, _) in &MIME_TYPES {
            assert!(ext.is_ascii(), "Extension not ASCII: {:?}", ext);
        }
    }
}

#[cfg(feature = "bench")]
mod bench {
    extern crate test;

    use self::test::Bencher;

    use super::{get_mime_type_str, MIME_TYPES};

    /// WARNING: this may take a while!
    #[bench]
    fn bench_mime_str(b: &mut Bencher) {
        for (mime_ext, _) in &MIME_TYPES {
            b.iter(|| {
                get_mime_type_str(mime_ext).expect(mime_ext);
            });
        }
    }
}
