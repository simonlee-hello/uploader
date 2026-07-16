/// Base58 encode matching wenshushu verify.js / bs58 (Bitcoin alphabet).
/// Note: older Go utils.Base58Encode truncates and must NOT be used for wss a-code.
pub fn encode(src: &[u8]) -> String {
    const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    if src.is_empty() {
        return String::new();
    }

    let mut zeros = 0usize;
    while zeros < src.len() && src[zeros] == 0 {
        zeros += 1;
    }

    // size = input * log(256)/log(58) + 1
    let size = (src.len() - zeros) * 138 / 100 + 1;
    let mut buf = vec![0u8; size];
    let mut length = 0usize;

    for &byte in &src[zeros..] {
        let mut carry = byte as u32;
        let mut j = 0usize;
        let mut i = size;
        while i > 0 && (carry != 0 || j < length) {
            i -= 1;
            carry += 256 * (buf[i] as u32);
            buf[i] = (carry % 58) as u8;
            carry /= 58;
            j += 1;
        }
        length = j;
    }

    let mut i = size - length;
    while i < size && buf[i] == 0 {
        i += 1;
    }

    let mut out = Vec::with_capacity(zeros + (size - i));
    for _ in 0..zeros {
        out.push(ALPHABET[0]);
    }
    while i < size {
        out.push(ALPHABET[buf[i] as usize]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_hello() {
        assert_eq!(encode(b"hello"), "Cn8eVZg");
    }

    #[test]
    fn encode_md5_hex_matches_wss_verify_js() {
        let hex = b"7ceb0b114c8719bf3ac178a9e8ebb7f7";
        assert_eq!(
            encode(hex),
            "4jDL8D8X1p6q7mBMiR2MfjMCBHWu7hapwC2gLL2jFs7c"
        );
    }
}
