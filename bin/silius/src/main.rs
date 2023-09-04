fn main() {
    if let Err(err) = silius::cli::run() {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}
