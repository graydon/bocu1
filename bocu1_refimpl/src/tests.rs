use crate::RefImplEncodeBOCU1;
use bocu1::EncodeBOCU1;
extern crate quickcheck;

fn check_conforming(s: &str) {
    let v: Vec<u8> = s.refimpl_encode_bocu1().collect();
    let u: Vec<u8> = s.encode_bocu1().collect();
    assert_eq!(v, u);
}

#[test]
fn test_english() {
    check_conforming(&"hello");
}

#[test]
fn test_chinese() {
    check_conforming(&"學而時習之");
}

#[test]
fn test_katakana() {
    check_conforming(&"コンニチワ");
}

#[test]
fn test_100k_random_strings() {
    use self::quickcheck::*;
    fn check_one(s: String) -> bool {
        let v: Vec<u8> = s.as_str().refimpl_encode_bocu1().collect();
        let u: Vec<u8> = s.as_str().encode_bocu1().collect();
        u == v
    }
    QuickCheck::new()
        .tests(100_000)
        .max_tests(100_000)
        .quickcheck(check_one as fn(String) -> bool)
}
