use std::{
    collections::BTreeMap,
    fmt, io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};

use byteorder::BigEndian;
use getset::{CopyGetters, Getters};
use serde::Serialize;
use tokio::io::{AsyncRead, AsyncWrite};

pub fn decode_bencoded_value(encoded_value: &[u8]) -> (Value, usize) {
    // If encoded_value starts with a digit, it's a number
    if encoded_value[0].is_ascii_digit() {
        // Example: "5:hello" -> "hello"
        let colon_index = encoded_value.iter().position(|v| *v == b':').unwrap();
        let number_string = &encoded_value[..colon_index];
        let number = String::from_utf8_lossy(number_string)
            .parse::<i64>()
            .unwrap();
        let read = colon_index + 1 + number as usize;
        let string = &encoded_value[colon_index + 1..read];
        return (Value::Bytes(string.to_owned()), read);
    }

    // If encoded_value starts with 'i', it's an integer
    if encoded_value[0] == b'i' {
        // Example: "i52e" -> 52
        // Example: "i-52e" -> -52
        let e_index = encoded_value.iter().position(|v| *v == b'e').unwrap();
        let integer_string = &encoded_value[1..e_index];
        let integer = String::from_utf8_lossy(integer_string)
            .parse::<i64>()
            .unwrap();
        return (Value::Integer(integer), e_index + 1);
    }

    // If encoded_value starts with 'l', it's a list
    if encoded_value[0] == b'l' {
        // Example: "l5:helloi52ee" -> ["hello", 52]
        let mut elements = vec![];
        let mut pos = 1;
        loop {
            let remaining = encoded_value.get(pos..).unwrap();
            if remaining[0] == b'e' {
                return (Value::List(elements), pos);
            }
            let (element, read) = decode_bencoded_value(remaining);
            elements.push(element);
            pos += read;
        }
    }

    // If encoded_value starts with 'd', it's a dictionary
    if encoded_value[0] == b'd' {
        // Example: "d3:foo3:bar5:helloi52ee" -> {"hello": 52, "foo":"bar"}
        let mut map: BTreeMap<String, Value> = Default::default();
        let mut pos = 1;
        loop {
            let remaining = encoded_value.get(pos..).unwrap();
            if remaining[0] == b'e' {
                return (Value::Dictionary(map), pos);
            }
            let (key, read) = decode_bencoded_value(remaining);
            let key = match key {
                Value::Bytes(string) => String::from_utf8(string).unwrap(),
                _ => panic!(),
            };
            pos += read;

            let remaining = encoded_value.get(pos..).unwrap();
            let (value, read) = decode_bencoded_value(remaining);
            pos += read;

            map.insert(key, value);
        }
    }

    panic!(
        "Unhandled encoded value: {}",
        String::from_utf8_lossy(encoded_value)
    )
}

pub fn encode_bencoded_value(decoded_value: &Value) -> Vec<u8> {
    let mut encoded_value = vec![];
    match decoded_value {
        Value::Bytes(bytes) => {
            let length = bytes.len().to_string();
            encoded_value.extend(length.bytes());
            encoded_value.push(b':');
            encoded_value.extend(bytes);
        }
        Value::Integer(integer) => {
            encoded_value.push(b'i');
            let integer = integer.to_string();
            encoded_value.extend(integer.bytes());
            encoded_value.push(b'e');
        }
        Value::List(list) => {
            encoded_value.push(b'l');
            for item in list {
                let item = encode_bencoded_value(item);
                encoded_value.extend(item);
            }
            encoded_value.push(b'e');
        }
        Value::Dictionary(dictionary) => {
            encoded_value.push(b'd');
            for (key, value) in dictionary {
                let key = encode_bencoded_value(&Value::Bytes(key.as_bytes().into()));
                encoded_value.extend(key);
                let value = encode_bencoded_value(value);
                encoded_value.extend(value);
            }
            encoded_value.push(b'e');
        }
    }
    encoded_value
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    #[test]
    fn test_string() {
        let encoded_value = b"5:hello";
        let (value, _) = decode_bencoded_value(encoded_value);
        assert_eq!(value, Value::Bytes(b"hello".into()));
        assert_eq!(encoded_value, &encode_bencoded_value(&value)[..]);
    }

    #[test]
    fn test_number() {
        let encoded_value = b"i52e";
        let (value, _) = decode_bencoded_value(encoded_value);
        assert_eq!(value, Value::Integer(52));
        assert_eq!(encoded_value, &encode_bencoded_value(&value)[..]);

        let encoded_value = b"i-52e";
        let (value, _) = decode_bencoded_value(encoded_value);
        assert_eq!(value, Value::Integer(-52));
        assert_eq!(encoded_value, &encode_bencoded_value(&value)[..]);
    }

    #[test]
    fn test_list() {
        let encoded_value = b"l5:helloi52ee";
        let (value, _) = decode_bencoded_value(encoded_value);
        assert_eq!(
            value,
            Value::List(vec![
                Value::Bytes("hello".into()),
                Value::Integer(52.into()),
            ])
        );
        assert_eq!(encoded_value, &encode_bencoded_value(&value)[..]);
    }

    #[test]
    fn test_dictionary() {
        let encoded_value = b"d3:foo3:bar5:helloi52ee";
        let (value, _) = decode_bencoded_value(encoded_value);
        let mut map = BTreeMap::new();
        map.insert("hello".into(), Value::Integer(52));
        map.insert("foo".into(), Value::Bytes(b"bar".into()));
        assert_eq!(value, Value::Dictionary(map));
        assert_eq!(encoded_value, &encode_bencoded_value(&value)[..]);
    }

    #[test]
    fn test_metainfo() {
        let file = "sample.torrent";
        let mut file = std::fs::File::options().read(true).open(file).unwrap();
        let mut buf = vec![];
        file.read_to_end(&mut buf).unwrap();
        let (_value, _) = decode_bencoded_value(&buf);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Value {
    Bytes(Vec<u8>),
    Integer(i64),
    List(Vec<Self>),
    Dictionary(BTreeMap<String, Self>),
}

impl Value {
    pub fn into_bytes(self) -> Option<Vec<u8>> {
        let Self::Bytes(bytes) = self else {
            return None;
        };
        Some(bytes)
    }

    pub fn into_integer(self) -> Option<i64> {
        let Self::Integer(integer) = self else {
            return None;
        };
        Some(integer)
    }

    pub fn into_list(self) -> Option<Vec<Self>> {
        let Self::List(list) = self else {
            return None;
        };
        Some(list)
    }

    pub fn into_dictionary(self) -> Option<BTreeMap<String, Self>> {
        let Self::Dictionary(dictionary) = self else {
            return None;
        };
        Some(dictionary)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // fn fmt_value(v: &Value, f: &mut fmt::Formatter<'_>, indents: usize) -> fmt::Result {}
        match self {
            Value::Bytes(bytes) => write!(f, "r#\"{}\"#", String::from_utf8_lossy(bytes))?,
            Value::Integer(integer) => write!(f, "{integer}")?,
            Value::List(list) => {
                write!(f, "[")?;
                for (i, element) in list.iter().enumerate() {
                    write!(f, "{element}")?;
                    if i + 1 < list.len() {
                        write!(f, ",")?;
                    }
                }
                write!(f, "]")?;
            }
            Value::Dictionary(dictionary) => {
                write!(f, "{{")?;
                for (i, (key, value)) in dictionary.iter().enumerate() {
                    write!(f, "\"{key}\":{value}")?;
                    if i + 1 < dictionary.len() {
                        write!(f, ",")?;
                    }
                }
                write!(f, "}}")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Getters)]
pub struct Metainfo {
    #[getset(get = "pub")]
    announce: String,
    #[getset(get = "pub")]
    info: MetainfoInfo,
}

impl Metainfo {
    pub fn decode(value: Value) -> Self {
        let mut value = value.into_dictionary().unwrap();
        let announce =
            String::from_utf8(value.remove("announce").unwrap().into_bytes().unwrap()).unwrap();
        let info = MetainfoInfo::decode(value.remove("info").unwrap());
        Self { announce, info }
    }
}

#[derive(Debug, Getters, CopyGetters)]
pub struct MetainfoInfo {
    #[getset(get_copy = "pub")]
    length: usize,
    #[getset(get = "pub")]
    name: String,
    #[getset(get_copy = "pub")]
    piece_length: usize,
    pieces: Vec<u8>,
    #[getset(get = "pub")]
    hash: [u8; 20],
}

impl MetainfoInfo {
    pub fn decode(value: Value) -> Self {
        let bencoded = encode_bencoded_value(&value);
        use sha1::Digest;
        let mut hasher = sha1::Sha1::new();
        hasher.update(&bencoded);
        let hash = hasher.finalize().into();

        let mut value = value.into_dictionary().unwrap();
        let length = value.remove("length").unwrap().into_integer().unwrap();
        let name = String::from_utf8(value.remove("name").unwrap().into_bytes().unwrap()).unwrap();
        let piece_length = value
            .remove("piece length")
            .unwrap()
            .into_integer()
            .unwrap();
        let pieces = value.remove("pieces").unwrap().into_bytes().unwrap();
        Self {
            length: usize::try_from(length).unwrap(),
            name,
            piece_length: usize::try_from(piece_length).unwrap(),
            pieces,
            hash,
        }
    }

    pub fn piece_hashes(&self) -> impl Iterator<Item = &[u8]> {
        self.pieces.chunks(20)
    }
}

pub struct TrackerRequest<'caller> {
    pub info_hash: &'caller [u8],
    pub peer_id: &'caller [u8],
    pub port: u16,
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub compact: bool,
}

impl<'a> TrackerRequest<'a> {
    pub fn url(&'a self, metainfo: &'a Metainfo) -> String {
        let url_encoded_info_hash = urlencoding::encode_binary(metainfo.info().hash());
        let url_encoded_peer_id = urlencoding::encode_binary(self.peer_id);

        let mut url = String::new();
        url.push_str(metainfo.announce());
        url.push('?');
        url.push_str("info_hash=");
        url.push_str(&url_encoded_info_hash);
        url.push('&');
        url.push_str("peer_id=");
        url.push_str(&url_encoded_peer_id);
        url.push('&');
        url.push_str("port=");
        url.push_str(&self.port.to_string());
        url.push('&');
        url.push_str("uploaded=");
        url.push_str(&self.uploaded.to_string());
        url.push('&');
        url.push_str("downloaded=");
        url.push_str(&self.downloaded.to_string());
        url.push('&');
        url.push_str("left=");
        url.push_str(&self.left.to_string());
        url.push('&');
        url.push_str("compact=");
        url.push_str(&(self.compact as u8).to_string());
        url
    }
}

#[derive(Debug, Getters, CopyGetters)]
pub struct TrackerResponse {
    #[getset(get_copy = "pub")]
    interval: u64,
    #[getset(get = "pub")]
    peers: Vec<SocketAddr>,
}

impl TrackerResponse {
    pub fn decode(value: Value) -> Self {
        let mut value = value.into_dictionary().unwrap();
        let interval =
            u64::try_from(value.remove("interval").unwrap().into_integer().unwrap()).unwrap();
        let peers = value.remove("peers").unwrap().into_bytes().unwrap();
        let peers = peers.chunks_exact(6);
        let peers = peers
            .map(|bytes| {
                use byteorder::ReadBytesExt;
                let mut reader = io::Cursor::new(bytes);
                let _ip = reader.read_u32::<BigEndian>().unwrap();
                let port = reader.read_u16::<BigEndian>().unwrap();
                SocketAddr::V4(SocketAddrV4::new(
                    Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]),
                    port,
                ))
            })
            .collect();

        Self { interval, peers }
    }
}

#[derive(Debug, Getters)]
pub struct HandshakeResponse {
    #[getset(get = "pub")]
    info_hash: [u8; 20],
    #[getset(get = "pub")]
    peer_id: [u8; 20],
}

impl HandshakeResponse {
    pub async fn decode<R>(reader: &mut R) -> Self
    where
        R: AsyncRead + Unpin,
    {
        use tokio::io::AsyncReadExt;
        let length = reader.read_u8().await.unwrap();
        let mut protocol = vec![0; length as usize];
        reader.read_exact(&mut protocol).await.unwrap();
        assert_eq!("BitTorrent protocol", String::from_utf8(protocol).unwrap());
        let mut reserved = [0; 8];
        reader.read_exact(&mut reserved).await.unwrap();
        let mut info_hash = [0; 20];
        reader.read_exact(&mut info_hash).await.unwrap();
        let mut peer_id = [0; 20];
        reader.read_exact(&mut peer_id).await.unwrap();
        Self { info_hash, peer_id }
    }
}

pub struct HandshakeRequest<'caller> {
    pub info_hash: &'caller [u8; 20],
    pub peer_id: &'caller [u8; 20],
}

impl HandshakeRequest<'_> {
    pub async fn encode<W>(&self, writer: &mut W)
    where
        W: AsyncWrite + Unpin,
    {
        use tokio::io::AsyncWriteExt;
        let protocol = b"BitTorrent protocol";
        writer.write_u8(protocol.len() as u8).await.unwrap();
        writer.write_all(protocol).await.unwrap();
        writer.write_all(&[0; 8]).await.unwrap();
        writer.write_all(self.info_hash).await.unwrap();
        writer.write_all(self.peer_id).await.unwrap();
        writer.flush().await.unwrap();
    }
}
