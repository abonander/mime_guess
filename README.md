# mime_guess [![Build Status](https://travis-ci.org/cybergeek94/mime_guess.svg?branch=master)](https://travis-ci.org/cybergeek94/mime_guess)

MIME/MediaType guessing by file extension. Uses a comprehensive static list of known file extension to MIME type associations with little copying.

Contributing
-----------

####Adding or correcting MIME types for extensions

Is the MIME type for a file extension wrong or missing? Great! Well, not great for us, but great for you if you'd like to open a pull request! 

The file extension -> MIME type mappings are listed in `src/mime_types.rs`. **The list is sorted alphabetically by file extension, and all extensions are lowercase (where applicable).** This is necessary for the search to work properly, and is covered by the test suite. 

Simply add or update the appropriate string pair(s) to make the correction(s) needed. Run `cargo test` to make sure the library continues to work correctly.

####(Important!) Citing the corrected MIME type 

When opening a pull request, please include a link to an official document or RFC noting the correct MIME type for the file type in question. Though we're only guessing here, we like to be as correct as we can. It makes it much easier to vet your contribution if we don't have to search for corroborating material.

####Changes to the API or operation of the crate

We're open to changes to the crate's API or its inner workings, breaking or not, if it improves the overall operation, efficiency, or ergonomics of the crate. However, it would be a good idea to open an issue on the repository so we can discuss your proposed changes and decide how best to approach them.


License
-------

MIT (See the `LICENSE` file in this repository for more information.)
