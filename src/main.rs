// use serde_json;
use std::{env, io::Read};

use bittorrent_starter_rust::decode_bencoded_value;

// Available if you need it!
// use serde_bencode;

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let (decoded_value, _) = decode_bencoded_value(encoded_value.as_bytes());
        println!("{decoded_value}");
    } else if command == "info" {
        let file = &args[2];
        let mut file = std::fs::File::options().read(true).open(file).unwrap();
        let mut buf = vec![];
        file.read_to_end(&mut buf).unwrap();
        let (decoded_value, _) = decode_bencoded_value(&buf);
        println!("{decoded_value}");
    } else {
        println!("unknown command: {}", args[1])
    }
}
