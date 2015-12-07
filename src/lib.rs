//! Guessing of MIME types by file extension.
//!
//! Uses a static list of file-extension : MIME type mappings.

extern crate mime;

use mime::Mime;

pub use mime_types::MIME_TYPES;

use std::ascii::AsciiExt;
use std::ffi::OsStr;
use std::path::Path;

mod mime_types;

/// Guess the MIME type of the `Path` by its extension.
///
/// If the given `Path` has no extension, or its extension has no known MIME type mapping,
/// then the MIME type is assumed to be `application/octet-stream`.
///
/// ##Note
/// **Guess** is the operative word here, as there are no guarantees that the contents of the file
/// that `path` points to match the MIME type associated with the path's extension.
///
/// Take care when processing files with assumptions based on the return value of this function.
pub fn guess_mime_type<P: AsRef<Path>>(path: P) -> Mime {
    let ext = path.as_ref().extension().and_then(OsStr::to_str).unwrap_or("");

    get_mime_type(ext)
}

/// Get the MIME type associated with a file extension.
///
/// If there is no association for the extension, or `ext` is empty,
/// `application/octet-stream` is returned.
pub fn get_mime_type(search_ext: &str) -> Mime {
    get_mime_type_str(search_ext)
        .map(|mime| mime.parse::<Mime>().unwrap())
        .unwrap_or_else(octet_stream)
}

/// Get the MIME type string associated with a file extension.
///
///
/// `search_ext` is converted to lowercase for a case-insensitive binary search.
///
/// Returns `None` if `search_ext` is empty or an associated extension was not found.
pub fn get_mime_type_str(search_ext: &str) -> Option<&'static str> {
    if search_ext.is_empty() { return None; }

    let search_ext = search_ext.to_ascii_lowercase();

    MIME_TYPES.binary_search_by(|&(ext, _)| ext.cmp(&search_ext))
        .ok().map(|idx| MIME_TYPES[idx].1)
}

/// Get the MIME type for `application/octet-stream` (generic binary stream)
pub fn octet_stream() -> Mime {
    "application/octet-stream".parse().unwrap()
}

#[cfg(test)]
mod tests {
    use mime::Mime;
    use std::path::Path;
    use super::{get_mime_type, guess_mime_type, MIME_TYPES};

    #[test]
    fn test_mime_type_guessing() {
        assert_eq!(get_mime_type("gif").to_string(), "image/gif".to_string());
        assert_eq!(get_mime_type("txt").to_string(), "text/plain".to_string());
        assert_eq!(get_mime_type("blahblah").to_string(), "application/octet-stream".to_string());

        assert_eq!(guess_mime_type(Path::new("/path/to/file.gif")).to_string(), "image/gif".to_string());
        assert_eq!(guess_mime_type("/path/to/file.gif").to_string(), "image/gif".to_string());
    }

    #[test]
    fn test_are_extensions_sorted() {
        // To make binary search work, extensions need to be sorted in ascending order.
    	for (curr, next) in MIME_TYPES.iter().zip(MIME_TYPES.iter().skip(1)) {
    		assert!(curr <= next, "MIME type mappings are not sorted! Failed assert: {:?} <= {:?}", curr, next);
    	}
    }

    #[test]
    fn test_are_mime_types_parseable() {
        for &(_, mime) in MIME_TYPES {
            mime.parse::<Mime>().unwrap();
        }
    }
}
