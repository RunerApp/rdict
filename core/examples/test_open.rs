fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or("headwords/eng-zho-ngsl.rdict".into());
    match rdict::RdictReader::open(&path) {
        Ok(reader) => {
            println!(
                "OK: opened, {} entries",
                reader.manifest().index.entry_count
            );
        }
        Err(e) => println!("ERROR: {}", e),
    }
}
