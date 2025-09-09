// simple example illustrating how diffent extensions
// can match zero or multiple mime types

fn print_guess_from_path(path: &str) {
    let guess: mime_guess::MimeGuess = mime_guess::from_path(path);
    if guess.count() == 0 {
        println!("unable to guess mime type from path: {}", path);
    } else {
        println!("guessing from path: {}", path);
        guess.iter().for_each(|s| {
            println!("  mime: {}", s);
        });
    }
}

fn main() {
    print_guess_from_path("/path/to/file");
    print_guess_from_path("/path/to/file.gif");
    print_guess_from_path("/path/to/file.md");
}
