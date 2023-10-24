// use serde_json;
use std::{env, io::Read};

// Available if you need it!
// use serde_bencode;

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &[u8]) -> (serde_json::Value, usize) {
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
        return (
            serde_json::Value::String(String::from_utf8_lossy(string).into()),
            read,
        );
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
        return (serde_json::Value::Number(integer.into()), e_index + 1);
    }

    // If encoded_value starts with 'l', it's a list
    if encoded_value[0] == b'l' {
        // Example: "l5:helloi52ee" -> ["hello", 52]
        let mut elements = vec![];
        let mut pos = 1;
        loop {
            let remaining = encoded_value.get(pos..).unwrap();
            if remaining[0] == b'e' {
                return (serde_json::Value::Array(elements), pos);
            }
            let (element, read) = decode_bencoded_value(remaining);
            elements.push(element);
            pos += read;
        }
    }

    // If encoded_value starts with 'd', it's a dictionary
    if encoded_value[0] == b'd' {
        // Example: "d3:foo3:bar5:helloi52ee" -> {"hello": 52, "foo":"bar"}
        let mut map: serde_json::Map<String, serde_json::Value> = Default::default();
        let mut pos = 1;
        loop {
            let remaining = encoded_value.get(pos..).unwrap();
            if remaining[0] == b'e' {
                return (serde_json::Value::Object(map), pos);
            }
            let (key, read) = decode_bencoded_value(remaining);
            let key = key.as_str().unwrap().to_string();
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

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let (decoded_value, _) = decode_bencoded_value(encoded_value.as_bytes());
        println!("{}", decoded_value);
    } else if command == "info" {
        let file = &args[2];
        let mut file = std::fs::File::options().read(true).open(file).unwrap();
        let mut buf = vec![];
        file.read_to_end(&mut buf).unwrap();
        let (value, _) = decode_bencoded_value(&buf);
        dbg!(&value);
    } else {
        println!("unknown command: {}", args[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string() {
        let encoded_value = b"5:hello";
        let (value, _) = decode_bencoded_value(encoded_value);
        assert_eq!(value, serde_json::Value::String("hello".into()));
    }

    #[test]
    fn test_number() {
        let encoded_value = b"i52e";
        let (value, _) = decode_bencoded_value(encoded_value);
        assert_eq!(value, serde_json::Value::Number(52.into()));
        let encoded_value = b"i-52e";
        let (value, _) = decode_bencoded_value(encoded_value);
        assert_eq!(value, serde_json::Value::Number((-52).into()));
    }

    #[test]
    fn test_list() {
        let encoded_value = b"l5:helloi52ee";
        let (value, _) = decode_bencoded_value(encoded_value);
        assert_eq!(
            value,
            serde_json::Value::Array(vec![
                serde_json::Value::String("hello".into()),
                serde_json::Value::Number(52.into()),
            ])
        );
    }

    #[test]
    fn test_dictionary() {
        let encoded_value = b"d3:foo3:bar5:helloi52ee";
        let (value, _) = decode_bencoded_value(encoded_value);
        let mut map = serde_json::Map::new();
        map.insert("hello".into(), serde_json::Value::Number(52.into()));
        map.insert("foo".into(), serde_json::Value::String("bar".into()));
        assert_eq!(value, serde_json::Value::Object(map));
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
