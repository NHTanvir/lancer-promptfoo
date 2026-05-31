//! Deterministic text encoders for the static/low-cost strategy tier.
//!
//! These are pure functions — no LLM call, no I/O — so they are exercised by
//! unit tests and form the part of the strategy layer that is fully native
//! (the dynamic/multi-turn tiers need a model and are described, not encoded).

/// Standard Base64 (RFC 4648) with `=` padding.
pub fn base64_encode(input: &str) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Lowercase hex of the UTF-8 bytes.
pub fn hex_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.as_bytes() {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

/// ROT13 on ASCII letters; other characters pass through.
pub fn rot13(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'a'..='z' => (((c as u8 - b'a' + 13) % 26) + b'a') as char,
            'A'..='Z' => (((c as u8 - b'A' + 13) % 26) + b'A') as char,
            _ => c,
        })
        .collect()
}

/// Leetspeak substitution (a→4, e→3, i→1, o→0, s→5, t→7).
pub fn leetspeak(input: &str) -> String {
    input
        .chars()
        .map(|c| match c.to_ascii_lowercase() {
            'a' => '4',
            'e' => '3',
            'i' => '1',
            'o' => '0',
            's' => '5',
            't' => '7',
            _ => c,
        })
        .collect()
}

/// Reverse the string by chars (a trivial obfuscation).
pub fn reverse(input: &str) -> String {
    input.chars().rev().collect()
}

/// 8-bit binary, space-separated per byte.
pub fn binary_encode(input: &str) -> String {
    input
        .as_bytes()
        .iter()
        .map(|b| format!("{:08b}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Percent-encode every non-unreserved byte (URL encoding).
pub fn url_encode(input: &str) -> String {
    let mut out = String::new();
    for b in input.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// International Morse code; unknown chars dropped, words separated by `/`.
pub fn morse_encode(input: &str) -> String {
    fn code(c: char) -> Option<&'static str> {
        Some(match c.to_ascii_uppercase() {
            'A' => ".-",
            'B' => "-...",
            'C' => "-.-.",
            'D' => "-..",
            'E' => ".",
            'F' => "..-.",
            'G' => "--.",
            'H' => "....",
            'I' => "..",
            'J' => ".---",
            'K' => "-.-",
            'L' => ".-..",
            'M' => "--",
            'N' => "-.",
            'O' => "---",
            'P' => ".--.",
            'Q' => "--.-",
            'R' => ".-.",
            'S' => "...",
            'T' => "-",
            'U' => "..-",
            'V' => "...-",
            'W' => ".--",
            'X' => "-..-",
            'Y' => "-.--",
            'Z' => "--..",
            '0' => "-----",
            '1' => ".----",
            '2' => "..---",
            '3' => "...--",
            '4' => "....-",
            '5' => ".....",
            '6' => "-....",
            '7' => "--...",
            '8' => "---..",
            '9' => "----.",
            '.' => ".-.-.-",
            ',' => "--..--",
            '?' => "..--..",
            _ => return None,
        })
    }
    input
        .split_whitespace()
        .map(|word| {
            word.chars()
                .filter_map(code)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" / ")
}

/// Pig Latin (move leading consonant cluster to the end + "ay").
pub fn pig_latin(input: &str) -> String {
    fn is_vowel(c: char) -> bool {
        matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u')
    }
    input
        .split_whitespace()
        .map(|word| {
            let chars: Vec<char> = word.chars().collect();
            if chars.is_empty() {
                return String::new();
            }
            if is_vowel(chars[0]) {
                format!("{}way", word)
            } else {
                let first_vowel = chars.iter().position(|&c| is_vowel(c)).unwrap_or(chars.len());
                let (cons, rest) = word.split_at(first_vowel);
                format!("{}{}ay", rest, cons)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// camelCase: strip spaces, capitalise each word after the first.
pub fn camel_case(input: &str) -> String {
    let mut out = String::new();
    for (i, word) in input.split_whitespace().enumerate() {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            if i == 0 {
                out.extend(first.to_lowercase());
            } else {
                out.extend(first.to_uppercase());
            }
            out.push_str(chars.as_str());
        }
    }
    out
}

/// Homoglyph substitution — swap common Latin letters for confusable Unicode.
pub fn homoglyph(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'a' => 'а', // Cyrillic a
            'e' => 'е', // Cyrillic e
            'o' => 'о', // Cyrillic o
            'p' => 'р', // Cyrillic er
            'c' => 'с', // Cyrillic es
            'x' => 'х', // Cyrillic ha
            'y' => 'у', // Cyrillic u
            _ => c,
        })
        .collect()
}

/// Emoji variation-selector smuggling: hide the bytes of `input` inside the
/// invisible Unicode variation selectors trailing a carrier emoji. This mirrors
/// the upstream `emoji-smuggling` strategy (data hidden in VS16 sequences).
pub fn emoji_smuggle(input: &str) -> String {
    // Variation selectors: U+FE00..U+FE0F (16) and U+E0100..U+E01EF (240) → 256
    // code points, enough to encode one byte each.
    fn byte_to_selector(b: u8) -> char {
        let cp = if b < 16 {
            0xFE00 + b as u32
        } else {
            0xE0100 + (b as u32 - 16)
        };
        char::from_u32(cp).unwrap_or('\u{FE00}')
    }
    let mut out = String::from("😀");
    for b in input.as_bytes() {
        out.push(byte_to_selector(*b));
    }
    out
}

/// ASCII-smuggling via Unicode "tag" characters (U+E0000 block) — the classic
/// invisible-instruction-injection vector.
pub fn ascii_smuggle(input: &str) -> String {
    let mut out = String::new();
    for b in input.bytes() {
        if let Some(c) = char::from_u32(0xE0000 + b as u32) {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_known_vectors() {
        assert_eq!(base64_encode(""), "");
        assert_eq!(base64_encode("f"), "Zg==");
        assert_eq!(base64_encode("fo"), "Zm8=");
        assert_eq!(base64_encode("foo"), "Zm9v");
        assert_eq!(base64_encode("foob"), "Zm9vYg==");
        assert_eq!(base64_encode("fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode("foobar"), "Zm9vYmFy");
    }

    #[test]
    fn hex_and_rot13() {
        assert_eq!(hex_encode("AB"), "4142");
        assert_eq!(rot13("Hello"), "Uryyb");
        assert_eq!(rot13(rot13("Hello").as_str()), "Hello");
    }

    #[test]
    fn morse_basic() {
        assert_eq!(morse_encode("SOS"), "... --- ...");
        assert_eq!(morse_encode("HI THERE").split('/').count(), 2);
    }

    #[test]
    fn leet_and_reverse() {
        assert_eq!(leetspeak("test"), "7e57");
        assert_eq!(reverse("abc"), "cba");
    }

    #[test]
    fn camel_and_url() {
        assert_eq!(camel_case("ignore all instructions"), "ignoreAllInstructions");
        assert_eq!(url_encode("a b"), "a%20b");
    }
}
