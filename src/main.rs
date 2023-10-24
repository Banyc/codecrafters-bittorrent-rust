// use serde_json;
use std::env;

// Available if you need it!
// use serde_bencode

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> (serde_json::Value, usize) {
    // If encoded_value starts with a digit, it's a number
    if encoded_value.chars().next().unwrap().is_ascii_digit() {
        // Example: "5:hello" -> "hello"
        let colon_index = encoded_value.find(':').unwrap();
        let number_string = &encoded_value[..colon_index];
        let number = number_string.parse::<i64>().unwrap();
        let read = colon_index + 1 + number as usize;
        let string = &encoded_value[colon_index + 1..read];
        return (serde_json::Value::String(string.to_string()), read);
    }

    // If encoded_value starts with 'i', it's an integer
    if encoded_value.starts_with('i') {
        // Example: "i52e" -> 52
        // Example: "i-52e" -> -52
        let e_index = encoded_value.find('e').unwrap();
        let integer_string = &encoded_value[1..e_index];
        let integer = integer_string.parse::<i64>().unwrap();
        return (serde_json::Value::Number(integer.into()), e_index + 1);
    }

    // If encoded_value starts with 'l', it's a list
    if encoded_value.starts_with('l') {
        // Example: "l5:helloi52ee" -> ["hello", 52]
        let mut elements = vec![];
        let mut pos = 1;
        loop {
            let remaining = encoded_value.get(pos..).unwrap();
            if remaining.starts_with('e') {
                return (serde_json::Value::Array(elements), pos);
            }
            let (element, read) = decode_bencoded_value(remaining);
            elements.push(element);
            pos += read;
        }
    }

    panic!("Unhandled encoded value: {}", encoded_value)
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let (decoded_value, _) = decode_bencoded_value(encoded_value);
        println!("{}", decoded_value);
    } else {
        println!("unknown command: {}", args[1])
    }
}
