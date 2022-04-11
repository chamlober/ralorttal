fn main() {
    match std::env::args().nth(1) {
        Some(path) => {
            if let Err(err) = transactions::process_csv(path) {
                eprintln!("Error: {}", err);
                std::process::exit(1);
            }
        }
        None => {
            eprintln!("Error: No CSV path provided");
            std::process::exit(1);
        }
    }
}
