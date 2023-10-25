// use serde_json;
use std::{env, fmt, io::Read};

use bittorrent_starter_rust::{decode_bencoded_value, Metainfo};

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
        // println!("{decoded_value}");
        let info = Metainfo::decode(decoded_value);
        println!("Tracker URL: {}", info.announce());
        println!("Length: {}", info.info().length());
        println!("Info Hash: {}", DisplayHash::from(&info.info().hash()[..]));
        println!("Piece Length: {}", info.info().piece_length());
        println!("Piece hashes:");
        for piece_hash in info.info().piece_hashes() {
            println!("{}", DisplayHash::from(piece_hash))
        }
    } else {
        println!("unknown command: {}", args[1])
    }
}

pub struct DisplayHash<'hash> {
    hash: &'hash [u8],
}

impl<'hash> From<&'hash [u8]> for DisplayHash<'hash> {
    fn from(value: &'hash [u8]) -> Self {
        Self { hash: value }
    }
}

impl fmt::Display for DisplayHash<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.hash {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}
