use std::{collections::BTreeMap, fmt};

use getset::{CopyGetters, Getters};

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

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    #[test]
    fn test_string() {
        let encoded_value = b"5:hello";
        let (value, _) = decode_bencoded_value(encoded_value);
        assert_eq!(value, Value::Bytes(b"hello".into()));
    }

    #[test]
    fn test_number() {
        let encoded_value = b"i52e";
        let (value, _) = decode_bencoded_value(encoded_value);
        assert_eq!(value, Value::Integer(52));
        let encoded_value = b"i-52e";
        let (value, _) = decode_bencoded_value(encoded_value);
        assert_eq!(value, Value::Integer(-52));
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
    }

    #[test]
    fn test_dictionary() {
        let encoded_value = b"d3:foo3:bar5:helloi52ee";
        let (value, _) = decode_bencoded_value(encoded_value);
        let mut map = BTreeMap::new();
        map.insert("hello".into(), Value::Integer(52));
        map.insert("foo".into(), Value::Bytes(b"bar".into()));
        assert_eq!(value, Value::Dictionary(map));
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
    #[getset(get = "pub")]
    pieces: Vec<u8>,
}

impl MetainfoInfo {
    pub fn decode(value: Value) -> Self {
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
        }
    }
}
