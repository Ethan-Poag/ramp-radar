//! Algorithmic US N-number <-> ICAO 24-bit address conversion.
//!
//! The FAA allocates ICAO addresses A00001..=ADF7C7 to N-numbers in a fixed
//! lexicographic walk, so no lookup table is needed. Layout of a subtree:
//! the bare prefix, then 24 single letters + 24*24 two-letter suffixes
//! (letters exclude I and O), then ten digit sub-trees. After four digits the
//! 5th character is either a letter (24) or a digit (10).

const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ"; // 24 letters, no I/O
const BASE: u32 = 0xA0_0001;
const SUFFIX: u32 = 1 + 24 * 25; // bare + (letter + letter,letter)  = 601
const S4: u32 = 1 + 24 + 10; // after 4 digits = 35
const S3: u32 = SUFFIX + 10 * S4; // 951
const S2: u32 = SUFFIX + 10 * S3; // 10111
const S1: u32 = SUFFIX + 10 * S2; // 101711
const LAST: u32 = BASE + 9 * S1 - 1; // 0xADF7C7

fn letter_idx(c: u8) -> Option<u32> {
    CHARSET.iter().position(|&x| x == c).map(|i| i as u32)
}

/// Offset of a 1–2 letter suffix inside a subtree (bare prefix = 0).
fn suffix_offset(s: &[u8]) -> Option<u32> {
    match s {
        [a] => Some(1 + letter_idx(*a)? * 25),
        [a, b] => Some(1 + letter_idx(*a)? * 25 + 1 + letter_idx(*b)?),
        _ => None,
    }
}

/// "N12345" / "n12345" -> ICAO hex (uppercase, 6 chars). `None` if not a valid N-number.
pub fn n_to_icao(reg: &str) -> Option<String> {
    let reg = reg.trim().to_ascii_uppercase();
    let body = reg.strip_prefix('N')?.as_bytes();
    if body.is_empty() || body.len() > 5 {
        return None;
    }
    // first char: digit 1-9
    if !(b'1'..=b'9').contains(&body[0]) {
        return None;
    }
    let mut out = BASE + (body[0] - b'1') as u32 * S1;
    let sizes = [S2, S3, S4]; // subtree sizes for char index 1,2,3
    let mut i = 1;
    while i < body.len() {
        let c = body[i];
        if i == 4 {
            // last char: letter or digit
            if c.is_ascii_digit() {
                out += 25 + (c - b'0') as u32;
            } else {
                out += 1 + letter_idx(c)?;
            }
            i += 1;
        } else if c.is_ascii_digit() {
            out += SUFFIX + (c - b'0') as u32 * sizes[i - 1];
            i += 1;
        } else {
            // letters: only 1-2 allowed, must end the string
            out += suffix_offset(&body[i..])?;
            i = body.len();
        }
    }
    Some(format!("{out:06X}"))
}

/// ICAO hex -> "N12345". `None` if outside the N-number block.
pub fn icao_to_n(hex: &str) -> Option<String> {
    let v = u32::from_str_radix(hex.trim(), 16).ok()?;
    if !(BASE..=LAST).contains(&v) {
        return None;
    }
    let mut off = v - BASE;
    let mut s = String::from("N");
    s.push((b'1' + (off / S1) as u8) as char);
    off %= S1;
    let sizes = [S2, S3, S4];
    for lvl in 0..3 {
        if off == 0 {
            return Some(s);
        }
        if off < SUFFIX {
            let r = off - 1;
            let a = r / 25;
            let b = r % 25;
            s.push(CHARSET[a as usize] as char);
            if b > 0 {
                s.push(CHARSET[(b - 1) as usize] as char);
            }
            return Some(s);
        }
        off -= SUFFIX;
        s.push((b'0' + (off / sizes[lvl]) as u8) as char);
        off %= sizes[lvl];
    }
    // after 4 digits
    if off == 0 {
        return Some(s);
    }
    if off <= 24 {
        s.push(CHARSET[(off - 1) as usize] as char);
    } else {
        s.push((b'0' + (off - 25) as u8) as char);
    }
    Some(s)
}

/// Does this look like a US N-number (so that hex can be derived)?
pub fn looks_like_n_number(s: &str) -> bool {
    n_to_icao(s).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_values() {
        assert_eq!(n_to_icao("N1").unwrap(), "A00001");
        assert_eq!(n_to_icao("N1A").unwrap(), "A00002");
        assert_eq!(n_to_icao("N1AA").unwrap(), "A00003");
        assert_eq!(n_to_icao("N1B").unwrap(), "A0001B");
        assert_eq!(n_to_icao("N12345").unwrap(), "A061D9");
        assert_eq!(n_to_icao("N99999").unwrap(), "ADF7C7");
        assert_eq!(n_to_icao("n800bs").unwrap(), n_to_icao("N800BS").unwrap());
    }
    #[test]
    fn known_aircraft() {
        // N707QS (Netjets CL35) is seen live as a96ffe in the adsb.lol sample used in dev.
        assert_eq!(icao_to_n("A96FFE").unwrap(), "N707QS");
        assert_eq!(n_to_icao("N707QS").unwrap(), "A96FFE");
    }
    #[test]
    fn reverse_known() {
        assert_eq!(icao_to_n("a061d9").unwrap(), "N12345");
        assert_eq!(icao_to_n("ADF7C7").unwrap(), "N99999");
        assert_eq!(icao_to_n("A00001").unwrap(), "N1");
    }
    #[test]
    fn out_of_range() {
        assert!(icao_to_n("A00000").is_none());
        assert!(icao_to_n("ADF7C8").is_none());
        assert!(icao_to_n("zzzzzz").is_none());
    }
    #[test]
    fn invalid_regs() {
        for bad in ["", "N", "N0", "N1I", "N1O", "N123456", "G-ABCD", "N1ABC", "N12A3", "N1234AB", "12345"] {
            assert!(n_to_icao(bad).is_none(), "{bad}");
        }
    }
    #[test]
    fn exhaustive_roundtrip() {
        // every ICAO address in the block maps to a unique N-number and back
        let mut seen = std::collections::HashSet::new();
        for v in BASE..=LAST {
            let hex = format!("{v:06X}");
            let n = icao_to_n(&hex).unwrap_or_else(|| panic!("no n for {hex}"));
            assert_eq!(n_to_icao(&n).as_deref(), Some(hex.as_str()), "{n}");
            assert!(seen.insert(n));
        }
        assert_eq!(seen.len(), 915_399);
    }
}
