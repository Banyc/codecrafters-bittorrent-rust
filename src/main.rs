// use serde_json;
use std::{
    env, fmt,
    io::{self, Read},
    net::SocketAddr,
    path::Path,
};

use bittorrent_starter_rust::{
    decode_bencoded_value, HandshakeRequest, HandshakeResponse, Metainfo, PeerMessageId,
    PeerMessageIn, PeerMessageOut, PeerMessageRequest, PeerMessageResponse, TrackerRequest,
    TrackerResponse,
};
use tokio::net::TcpStream;

// Available if you need it!
// use serde_bencode;

// Usage: your_bittorrent.sh decode "<encoded_value>"
#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    let my_peer_id = b"00112233445566778899";
    let my_port = 6881;

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
            DisplayHex::from(&metainfo.info().hash()[..])
        );
        println!("Piece Length: {}", metainfo.info().piece_length());
        println!("Piece hashes:");
        for piece_hash in metainfo.info().piece_hashes() {
            println!("{}", DisplayHex::from(piece_hash))
        }
    } else if command == "peers" {
        let metainfo = parse_metainfo_file(&args[2]).unwrap();
        let resp = peers(&metainfo, my_peer_id, my_port).await;
        for peer in resp.peers() {
            println!("{peer}");
        }
    } else if command == "handshake" {
        let metainfo = parse_metainfo_file(&args[2]).unwrap();
        let peer = &args[3];
        let peer: SocketAddr = peer.parse().unwrap();
        let (_stream, handshake) = establish(&metainfo, my_peer_id, peer).await;
        println!("Peer ID: {}", DisplayHex::from(&handshake.peer_id()[..]));
    } else if command == "download_piece" {
        let metainfo = parse_metainfo_file(&args[4]).unwrap();
        let peers = peers(&metainfo, my_peer_id, my_port).await;
        let (mut stream, _handshake) = establish(&metainfo, my_peer_id, peers.peers()[0]).await;
        // println!(
        //     "{}, {}",
        //     stream.local_addr().unwrap(),
        //     stream.peer_addr().unwrap()
        // );
        // let mut line = String::new();
        // tokio::io::BufReader::new(tokio::io::stdin())
        //     .read_line(&mut line)
        //     .await
        //     .unwrap();
        let available_pieces = PeerMessageIn::decode(&mut stream).await;
        assert!(matches!(
            available_pieces.message_id(),
            PeerMessageId::Bitfield
        ));
        PeerMessageOut {
            message_id: PeerMessageId::Interested,
            payload: &[],
        }
        .encode(&mut stream)
        .await;
        let unchoke = PeerMessageIn::decode(&mut stream).await;
        assert!(matches!(unchoke.message_id(), PeerMessageId::Unchoke));
        // let piece_indices = args[5..].iter().map(|s| s.parse::<u32>().unwrap());
        let piece_index = args[5].parse::<u32>().unwrap();
        let block_size = 2_u32.pow(14);
        let output_file_path = &args[3];
        let _ = tokio::fs::remove_file(output_file_path).await;
        let mut output_file = tokio::fs::File::options()
            .write(true)
            .create(true)
            .open(output_file_path)
            .await
            .unwrap();
        // for piece_index in piece_indices {
        {
            let piece_length = metainfo
                .info()
                .piece_length()
                .min(metainfo.info().length() - metainfo.info().piece_length() * piece_index);

            let mut remaining_piece = piece_length;
            while remaining_piece > 0 {
                let begin = piece_length - remaining_piece;
                let block_size = remaining_piece.min(block_size);
                remaining_piece -= block_size;

                let req = PeerMessageRequest {
                    index: piece_index,
                    begin,
                    length: block_size,
                };
                let mut payload = vec![];
                req.encode(&mut payload).await;
                let req = PeerMessageOut {
                    message_id: PeerMessageId::Request,
                    payload: &payload,
                };
                req.encode(&mut stream).await;

                let resp = PeerMessageIn::decode(&mut stream).await;
                assert!(matches!(resp.message_id(), PeerMessageId::Piece));
                let payload_length = resp.payload().len();
                let mut payload = io::Cursor::new(resp.payload());
                let resp = PeerMessageResponse::decode(&mut payload, payload_length).await;
                assert_eq!(resp.block().len(), block_size as usize);
                use tokio::io::AsyncWriteExt;
                output_file.write_all(resp.block()).await.unwrap();
            }
        }
        println!("Piece {piece_index} downloaded to {output_file_path}");
    } else {
        println!("unknown command: {}", args[1])
    }
}

pub struct DisplayHex<'buf> {
    buf: &'buf [u8],
}

impl<'buf> From<&'buf [u8]> for DisplayHex<'buf> {
    fn from(value: &'buf [u8]) -> Self {
        Self { buf: value }
    }
}

impl fmt::Display for DisplayHex<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.buf {
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

async fn peers(metainfo: &Metainfo, my_peer_id: &[u8; 20], my_port: u16) -> TrackerResponse {
    let client = reqwest::Client::new();

    let req = TrackerRequest {
        info_hash: metainfo.info().hash(),
        peer_id: my_peer_id,
        port: my_port,
        uploaded: 0,
        downloaded: 0,
        left: metainfo.info().length() as u64,
        compact: true,
    };

    let url = req.url(metainfo);
    let resp = client.get(url).send().await.unwrap().bytes().await.unwrap();
    let (resp, _) = decode_bencoded_value(&resp);
    TrackerResponse::decode(resp)
}

async fn establish(
    metainfo: &Metainfo,
    my_peer_id: &[u8; 20],
    peer: SocketAddr,
) -> (TcpStream, HandshakeResponse) {
    let mut stream = TcpStream::connect(peer).await.unwrap();
    let handshake = HandshakeRequest {
        info_hash: metainfo.info().hash(),
        peer_id: my_peer_id,
    };
    handshake.encode(&mut stream).await;
    let handshake = HandshakeResponse::decode(&mut stream).await;
    (stream, handshake)
}
