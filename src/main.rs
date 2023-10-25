// use serde_json;
use std::{
    env, fmt,
    io::{self, Read},
    path::Path,
};

use bittorrent_starter_rust::{decode_bencoded_value, Metainfo, TrackerRequest, TrackerResponse};

// Available if you need it!
// use serde_bencode;

// Usage: your_bittorrent.sh decode "<encoded_value>"
#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let (decoded_value, _) = decode_bencoded_value(encoded_value.as_bytes());
        println!("{decoded_value}");
    } else if command == "info" {
        let metainfo = parse_metainfo_file(&args[2]).unwrap();
        println!("Tracker URL: {}", metainfo.announce());
        println!("Length: {}", metainfo.info().length());
        println!(
            "Info Hash: {}",
            DisplayHash::from(&metainfo.info().hash()[..])
        );
        println!("Piece Length: {}", metainfo.info().piece_length());
        println!("Piece hashes:");
        for piece_hash in metainfo.info().piece_hashes() {
            println!("{}", DisplayHash::from(piece_hash))
        }
    } else if command == "peers" {
        let metainfo = parse_metainfo_file(&args[2]).unwrap();
        let client = reqwest::Client::new();

        let req = TrackerRequest {
            info_hash: metainfo.info().hash(),
            peer_id: b"00112233445566778899",
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: metainfo.info().length() as u64,
            compact: true,
        };

        let url = req.url(&metainfo);
        let resp = client.get(url).send().await.unwrap().bytes().await.unwrap();
        let (resp, _) = decode_bencoded_value(&resp);
        let resp = TrackerResponse::decode(resp);
        for peer in resp.peers() {
            println!("{peer}");
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

fn parse_metainfo_file(path: impl AsRef<Path>) -> io::Result<Metainfo> {
    let mut file = std::fs::File::options().read(true).open(path)?;
    let mut buf = vec![];
    file.read_to_end(&mut buf)?;
    let (decoded_value, _) = decode_bencoded_value(&buf);
    Ok(Metainfo::decode(decoded_value))
}
