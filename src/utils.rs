use sha3::Digest;

static HEX_CHARS: [[u8; 2]; 255] = {
    let mut array = [[0, 0]; 255];
    array[b'0' as usize] = [1, 0];
    array[b'1' as usize] = [1, 1];
    array[b'2' as usize] = [1, 2];
    array[b'3' as usize] = [1, 3];
    array[b'4' as usize] = [1, 4];
    array[b'5' as usize] = [1, 5];
    array[b'6' as usize] = [1, 6];
    array[b'7' as usize] = [1, 7];
    array[b'8' as usize] = [1, 8];
    array[b'9' as usize] = [1, 9];
    array[b'a' as usize] = [1, 10];
    array[b'b' as usize] = [1, 11];
    array[b'c' as usize] = [1, 12];
    array[b'd' as usize] = [1, 13];
    array[b'e' as usize] = [1, 14];
    array[b'f' as usize] = [1, 15];
    array[b'A' as usize] = [1, 10];
    array[b'B' as usize] = [1, 11];
    array[b'C' as usize] = [1, 12];
    array[b'D' as usize] = [1, 13];
    array[b'E' as usize] = [1, 14];
    array[b'F' as usize] = [1, 15];
    array
};

static HEX_CHARS_ARRAY: &[u8] = b"0123456789ABCDEF";

pub fn to_hex_char(c: u8) -> char {
    return HEX_CHARS_ARRAY[c as usize] as char;
}

pub fn hex_str_to_bytes(hex: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut iter = hex.chars();
    loop {
        match (iter.next(), iter.next()) {
            (Some(a), Some(b)) => {
                let byte = u8::from_str_radix(&format!("{}{}", a, b), 16).unwrap();
                bytes.push(byte);
            }
            _ => break,
        }
    }

    bytes
}

pub fn bytes_to_hex_str(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        hex.push(to_hex_char(byte >> 4));
        hex.push(to_hex_char(byte & 0xF));
    }

    hex
}

pub fn to_checksum_address(address: &[u8; 20]) -> String {
    let address = bytes_to_hex_str(address);
    let hash_hex = bytes_to_hex_str(sha3::Keccak256::digest(address.as_bytes()).as_slice());
    let mut result = String::with_capacity(40);

    for (i, mut c) in address.chars().enumerate() {
        let num = hex_char_to_u8_unchecked(c);

        if num > 9 && hex_char_to_u8_unchecked(hash_hex.as_bytes()[i].into()) > 7 {
            c = c.to_ascii_uppercase();
        }

        result.push(c);
    }

    return result;
}

pub fn is_hex_char(c: char) -> bool {
    HEX_CHARS[c as usize][0] == 1
}

pub fn hex_char_to_u8_unchecked(c: char) -> u8 {
    HEX_CHARS[c as usize][1]
}

pub fn hex_char_to_u8(c: char) -> Option<u8> {
    if is_hex_char(c) {
        Some(HEX_CHARS[c as usize][1])
    } else {
        None
    }
}
