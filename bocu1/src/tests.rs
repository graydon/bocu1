use crate::packed::{pack, DecodePackedBOCU1};
use crate::DecodeBOCU1;
use crate::EncodeBOCU1;
use std::vec::Vec;
extern crate env_logger;
extern crate quickcheck;

fn check_roundtrip(s: &str, b: &[u8]) {
    let _ = env_logger::try_init();
    let v: Vec<u8> = s.encode_bocu1().collect();
    assert_eq!(v.as_slice(), b);
    let ch1: Vec<char> = s.chars().collect();
    let ch2: Vec<char> = v.as_slice().decode_bocu1().collect();
    assert_eq!(ch1, ch2);
}

#[test]
fn test_100k_random_strings() {
    use self::quickcheck::*;
    fn check_one(s: String) -> bool {
        let _ = env_logger::try_init();
        let _ = debug!("quickcheck: {:?}", s);
        let v: Vec<u8> = s.as_str().encode_bocu1().collect();
        let u: String = v.as_slice().decode_bocu1().collect();
        u == s
    }
    QuickCheck::new()
        .tests(100_000)
        .max_tests(100_000)
        .quickcheck(check_one as fn(String) -> bool)
}

#[test]
fn test_lex_order_50k_random_string_pairs() {
    use self::quickcheck::*;
    fn check_two(s1: String, s2: String) -> bool {
        // We're checking the lexicographic comparison preservation here.
        let _ = env_logger::try_init();
        let str_cmp = s1.cmp(&s2);
        let _ = debug!(
            "quickcheck lex order: {:?} vs {:?} == {:?}",
            s1, s2, str_cmp
        );
        let v1: Vec<u8> = s1.as_str().encode_bocu1().collect();
        let v2: Vec<u8> = s2.as_str().encode_bocu1().collect();
        let enc_cmp = v1.cmp(&v2);
        let _ = debug!(
            "quickcheck lex order encoded: {:?} vs {:?} == {:?}",
            v1, v2, enc_cmp
        );
        enc_cmp == str_cmp
    }
    QuickCheck::new()
        .tests(50_000)
        .max_tests(50_000)
        .quickcheck(check_two as fn(String, String) -> bool)
}

#[test]
fn test_lex_order_50k_random_packed_string_pairs() {
    use self::quickcheck::*;
    fn check_two(s1: String, s2: String) -> bool {
        // We're checking the _packed_ lexicographic comparison preservation here, which
        // means we truncate the strings to small enough to fit in a u128 before proceeding.
        // The worst-case string jumps from one end of the code range to the other on each
        // character, giving us a 4-byte coding unit. This means we can confidently put
        // at most 4 random unicode characters into a u128 (in practice it's almost always
        // more like 16, but this is worst-case and we're hooked up to a fuzzer, so..)
        let _ = env_logger::try_init();

        // Also note: NUL bytes are not allowed / respected at all in packed forms.
        let sc1: Vec<char> = s1.chars().filter(|x| *x != '\u{0}').collect();
        let sc2: Vec<char> = s2.chars().filter(|x| *x != '\u{0}').collect();
        let sc1_trunc = if sc1.len() > 4 { &sc1[0..4] } else { &sc1[..] };
        let sc2_trunc = if sc2.len() > 4 { &sc2[0..4] } else { &sc2[..] };
        let str_cmp = sc1_trunc.cmp(&sc2_trunc);
        let _ = debug!(
            "quickcheck packed lex order: {:?} vs {:?} == {:?}",
            sc1_trunc, sc2_trunc, str_cmp
        );
        let p1: u128 = pack(&sc1_trunc.iter()).unwrap();
        let p2: u128 = pack(&sc2_trunc.iter()).unwrap();
        let enc_cmp = p1.cmp(&p2);
        let _ = debug!(
            "quickcheck packed lex order encoded: {:?} vs {:?} == {:?}",
            p1, p2, enc_cmp
        );
        enc_cmp == str_cmp
    }
    QuickCheck::new()
        .tests(50_000)
        .max_tests(50_000)
        .quickcheck(check_two as fn(String, String) -> bool)
}

#[test]
fn test_english() {
    check_roundtrip(&"hello", &[0xb8, 0xb5, 0xbc, 0xbc, 0xbf]);
}

#[test]
fn test_chinese() {
    check_roundtrip(
        &"學而時習之",
        &[
            0xfb, 0x41, 0xd8, 0xd9, 0x3d, 0x3e, 0x94, 0xd8, 0xf6, 0x25, 0x58,
        ],
    );
}

#[test]
fn test_katakana() {
    check_roundtrip(&"コンニチワ", &[0xfb, 0x11, 0xca, 0xc3, 0x9b, 0x91, 0xbf]);
}

#[test]
fn test_hangul() {
    check_roundtrip(
        &"마인즈에서",
        &[
            0xfb, 0xa5, 0x3c, 0xd5, 0xb5, 0xd7, 0xdf, 0xd3, 0xf3, 0x4f, 0x8b,
        ],
    );
}

#[test]
fn test_arabic() {
    check_roundtrip(
        &"العالمية",
        &[0xd5, 0xf5, 0x94, 0x89, 0x77, 0x94, 0x95, 0x9a, 0x79],
    );
}

#[test]
fn test_hebrew() {
    check_roundtrip(
        &"הבינלאומי",
        &[0xd5, 0xa2, 0xa1, 0xa9, 0xb0, 0xac, 0xa0, 0xa5, 0xae, 0xa9],
    );
}

#[test]
fn test_cyrillic() {
    check_roundtrip(
        &"воплощению",
        &[
            0xd3, 0xe6, 0x8e, 0x8f, 0x8b, 0x8e, 0x99, 0x85, 0x8d, 0x88, 0x9e,
        ],
    );
}

#[test]
fn test_thai() {
    check_roundtrip(&"ธุรกิจ", &[0xde, 0x5b, 0x88, 0x73, 0x51, 0x84, 0x58]);
}

#[test]
fn test_devanagari() {
    check_roundtrip(&"आजकल", &[0xd8, 0xfb, 0x6c, 0x65, 0x82]);
}

#[test]
fn test_greek() {
    check_roundtrip(
        &"εφαρμογών",
        &[0xd3, 0x69, 0x96, 0x81, 0x91, 0x8c, 0x8f, 0x83, 0x9e, 0x8d],
    );
}

#[test]
fn test_multi() {
    check_roundtrip(
        &"hello εφαρμογών आजकल\n\
          воплощению HELLOコンニチワ\n",
        &[
            0xb8, 0xb5, 0xbc, 0xbc, 0xbf, 0x20, 0xd3, 0x69, 0x96, 0x81, 0x91, 0x8c, 0x8f, 0x83,
            0x9e, 0x8d, 0x20, 0xd5, 0x54, 0x6c, 0x65, 0x82, 0x0a, 0xd3, 0xe6, 0x8e, 0x8f, 0x8b,
            0x8e, 0x99, 0x85, 0x8d, 0x88, 0x9e, 0x20, 0x4c, 0x21, 0x95, 0x9c, 0x9c, 0x9f, 0xfb,
            0x11, 0xca, 0xc3, 0x9b, 0x91, 0xbf, 0x0a,
        ],
    );
}

#[test]
fn test_pack64() {
    let p: u64 = pack(&"hello").unwrap();
    assert_eq!(p, 0x_b8_b5_bc_bc__bf_00_00_00_u64);
}

#[test]
fn test_unpack64() {
    let p: u64 = 0x_b8_b5_bc_bc__bf_00_00_00_u64;
    let u: String = p.decode_packed_bocu1().collect();
    assert_eq!(u, "hello");
}

#[test]
fn test_pack128() {
    let p: u128 = pack(&"εφαρμογών").unwrap();
    assert_eq!(
        p,
        0x_d3_69_96_81__91_8c_8f_83__9e_8d_00_00__00_00_00_00_u128
    );
}

#[test]
fn test_unpack128() {
    let p: u128 = 0x_d3_69_96_81__91_8c_8f_83__9e_8d_00_00__00_00_00_00_u128;
    let u: String = p.decode_packed_bocu1().collect();
    assert_eq!(u, "εφαρμογών");
}

// This is some code to play with doing "exhaustive scans" of cartesian
// products across the whole unicode range, but that actually takes quite a
// while with even 2-char strings, so it's disabled for now. The
// quickchecker above is probably adequate.
/*
fn unicode_range(step: usize) -> impl Iterator<Item = char> {
    let lo = 0_u32 ..= 0xD7ff_u32;
    let hi = 0xE000_u32 ..= 0x10FFFF_u32;
    lo.into_iter().step_by(step)
        .chain(hi.into_iter().step_by(step))
        .map(|n| ::std::char::from_u32(n).unwrap())
}

#[test]
fn test_many_2_char_combinations() {
    for a in unicode_range(100) {
        for b in unicode_range(100) {
            let chars: [char;2] = [a, b];
            let slice = &chars[..];
            let v: Vec<u8> = slice.encode_bocu1().collect();
            let u: Vec<char> = v.as_slice().decode_bocu1().collect();
            assert_eq!(slice, u.as_slice());
        }
    }
}
*/
