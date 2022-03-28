fn main() {
    let path = std::env::args().nth(1).expect("No CSV path provided");
    if let Err(err) = transactions::process_csv(path) {
        println!("Error: {}", err);
        std::process::exit(1);
    }
}
